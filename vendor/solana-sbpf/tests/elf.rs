#![allow(clippy::literal_string_with_formatting_args)]

use byteorder::{ByteOrder, LittleEndian};
use solana_sbpf::{
    ebpf,
    elf::{get_ro_region, ElfError, Executable, Section},
    elf_parser::{
        consts::{ELFCLASS32, ELFCLASS64, ELFDATA2LSB, ELFDATA2MSB, ELFOSABI_NONE, EM_BPF, ET_REL},
        types::{Elf64Ehdr, Elf64Phdr, Elf64Shdr},
        Elf64, ElfParserError, SECTION_NAME_LENGTH_MAXIMUM,
    },
    memory_region::{AccessType, MemoryMapping},
    program::{BuiltinProgram, SBPFVersion},
    vm::Config,
};
use std::{fs::File, io::Read, sync::Arc};
use test_utils::{assert_error, syscalls, TestContextObject};

type ElfExecutable = Executable<TestContextObject>;

fn loader() -> Arc<BuiltinProgram<TestContextObject>> {
    let mut loader = BuiltinProgram::new_loader(Config::default());
    loader
        .register_function("log", syscalls::SyscallString::vm)
        .unwrap();
    loader
        .register_function("log_64", syscalls::SyscallU64::vm)
        .unwrap();
    Arc::new(loader)
}

#[test]
fn test_strict_header() {
    let elf_bytes = std::fs::read("tests/elfs/strict_header.so").expect("failed to read elf file");
    let loader = loader();

    // Check that the unmodified file can be parsed
    {
        let loader = Arc::new(BuiltinProgram::new_loader(Config {
            enable_symbol_and_section_labels: true,
            ..Config::default()
        }));
        let executable = ElfExecutable::load(&elf_bytes, loader.clone()).unwrap();
        let (name, _pc) = executable.get_function_registry().lookup_by_key(5).unwrap();
        assert_eq!(name, b"entrypoint");
    }

    // Check that using a reserved SBPF version fails
    {
        let mut elf_bytes = elf_bytes.clone();
        elf_bytes[0x0030] = 0xFF;
        let err = ElfExecutable::load(&elf_bytes, loader.clone()).unwrap_err();
        assert_eq!(err, ElfError::UnsupportedSBPFVersion);
    }

    // Check that an empty file fails
    let err = ElfExecutable::load_with_strict_parser(&[], loader.clone()).unwrap_err();
    assert_eq!(err, ElfParserError::OutOfBounds);

    // Break the file header one byte at a time
    let expected_results = std::iter::repeat_n(&Err(ElfParserError::InvalidFileHeader), 40)
        .chain(std::iter::repeat_n(&Ok(()), 12))
        .chain(std::iter::repeat_n(
            &Err(ElfParserError::InvalidFileHeader),
            8,
        ))
        .chain(std::iter::repeat_n(&Ok(()), 2))
        .chain(std::iter::repeat_n(
            &Err(ElfParserError::InvalidFileHeader),
            2,
        ));
    for (offset, expected) in (0..std::mem::size_of::<Elf64Ehdr>()).zip(expected_results) {
        let mut elf_bytes = elf_bytes.clone();
        elf_bytes[offset] = 0xAF;
        let result = ElfExecutable::load_with_strict_parser(&elf_bytes, loader.clone()).map(|_| ());
        assert_eq!(&result, expected);
    }

    // Break the program header table one byte at a time
    let expected_results_readonly =
        std::iter::repeat_n(&Err(ElfParserError::InvalidProgramHeader), 48)
            .chain(std::iter::repeat_n(&Ok(()), 8))
            .collect::<Vec<_>>();
    let expected_results_writable =
        std::iter::repeat_n(&Err(ElfParserError::InvalidProgramHeader), 40)
            .chain(std::iter::repeat_n(&Ok(()), 4))
            .chain(std::iter::repeat_n(
                &Err(ElfParserError::InvalidProgramHeader),
                4,
            ))
            .chain(std::iter::repeat_n(&Ok(()), 8))
            .collect::<Vec<_>>();
    let expected_results = vec![
        expected_results_readonly.iter(),
        expected_results_readonly.iter(),
        expected_results_writable.iter(),
        expected_results_writable.iter(),
    ];
    for (header_index, expected_results) in expected_results.into_iter().enumerate() {
        for (offset, expected) in (std::mem::size_of::<Elf64Ehdr>()
            + std::mem::size_of::<Elf64Phdr>() * header_index
            ..std::mem::size_of::<Elf64Ehdr>()
                + std::mem::size_of::<Elf64Phdr>() * (header_index + 1))
            .zip(expected_results)
        {
            let mut elf_bytes = elf_bytes.clone();
            elf_bytes[offset] = 0xAF;
            let result =
                ElfExecutable::load_with_strict_parser(&elf_bytes, loader.clone()).map(|_| ());
            assert_eq!(&&result, expected);
        }
    }

    // Check that an unaligned program header length fails
    {
        let mut elf_bytes = elf_bytes.clone();
        elf_bytes[0x60] = 0x29;
        elf_bytes[0x68] = 0x29;
        let err = ElfExecutable::load_with_strict_parser(&elf_bytes, loader.clone()).unwrap_err();
        assert_eq!(err, ElfParserError::InvalidProgramHeader);
    }

    // Check that an entrypoint missing a function start marker fails
    {
        let mut elf_bytes = elf_bytes.clone();
        elf_bytes[0x1B8] = 0x00;
        let err = ElfExecutable::load_with_strict_parser(&elf_bytes, loader.clone()).unwrap_err();
        assert_eq!(err, ElfParserError::InvalidFileHeader);
    }
}

#[test]
fn test_validate() {
    let elf_bytes = std::fs::read("tests/elfs/relative_call_sbpfv0.so").unwrap();
    let elf = Elf64::parse(&elf_bytes).unwrap();
    let mut header = elf.file_header().clone();

    let write_header = |header: Elf64Ehdr| unsafe {
        let mut bytes = elf_bytes.clone();
        std::ptr::write(bytes.as_mut_ptr().cast::<Elf64Ehdr>(), header);
        bytes
    };

    ElfExecutable::validate(&elf, &elf_bytes).expect("validation failed");

    header.e_ident.ei_class = ELFCLASS32;
    let bytes = write_header(header.clone());
    // the new parser rejects anything other than ELFCLASS64 directly
    Elf64::parse(&bytes).expect_err("allowed bad class");

    header.e_ident.ei_class = ELFCLASS64;
    let bytes = write_header(header.clone());
    ElfExecutable::validate(&Elf64::parse(&bytes).unwrap(), &elf_bytes).expect("validation failed");

    header.e_ident.ei_data = ELFDATA2MSB;
    let bytes = write_header(header.clone());
    // the new parser only supports little endian
    Elf64::parse(&bytes).expect_err("allowed big endian");

    header.e_ident.ei_data = ELFDATA2LSB;
    let bytes = write_header(header.clone());
    ElfExecutable::validate(&Elf64::parse(&bytes).unwrap(), &elf_bytes).expect("validation failed");

    header.e_ident.ei_osabi = 1;
    let bytes = write_header(header.clone());
    ElfExecutable::validate(&Elf64::parse(&bytes).unwrap(), &elf_bytes)
        .expect_err("allowed wrong abi");

    header.e_ident.ei_osabi = ELFOSABI_NONE;
    let bytes = write_header(header.clone());
    ElfExecutable::validate(&Elf64::parse(&bytes).unwrap(), &elf_bytes).expect("validation failed");

    header.e_machine = 42;
    let bytes = write_header(header.clone());
    ElfExecutable::validate(&Elf64::parse(&bytes).unwrap(), &elf_bytes)
        .expect_err("allowed wrong machine");

    header.e_machine = EM_BPF;
    let bytes = write_header(header.clone());
    ElfExecutable::validate(&Elf64::parse(&bytes).unwrap(), &elf_bytes).expect("validation failed");

    header.e_type = ET_REL;
    let bytes = write_header(header);
    ElfExecutable::validate(&Elf64::parse(&bytes).unwrap(), &elf_bytes)
        .expect_err("allowed wrong type");
}

#[test]
fn test_load() {
    let mut file = File::open("tests/elfs/relative_call_sbpfv0.so").expect("file open failed");
    let mut elf_bytes = Vec::new();
    file.read_to_end(&mut elf_bytes)
        .expect("failed to read elf file");
    ElfExecutable::load(&elf_bytes, loader()).expect("validation failed");
}

#[test]
fn test_load_unaligned() {
    let mut elf_bytes =
        std::fs::read("tests/elfs/relative_call_sbpfv0.so").expect("failed to read elf file");
    // The default allocator allocates aligned memory. Move the ELF slice to
    // elf_bytes.as_ptr() + 1 to make it unaligned and test unaligned
    // parsing.
    elf_bytes.insert(0, 0);
    ElfExecutable::load(&elf_bytes[1..], loader()).expect("validation failed");
}

#[test]
fn test_entrypoint() {
    let loader = loader();

    let mut file = File::open("tests/elfs/relative_call_sbpfv0.so").expect("file open failed");
    let mut elf_bytes = Vec::new();
    file.read_to_end(&mut elf_bytes)
        .expect("failed to read elf file");
    let elf = ElfExecutable::load(&elf_bytes, loader.clone()).expect("validation failed");
    let parsed_elf = Elf64::parse(&elf_bytes).unwrap();
    let executable: &Executable<TestContextObject> = &elf;
    assert_eq!(4, executable.get_entrypoint_instruction_offset());

    let write_header = |header: Elf64Ehdr| unsafe {
        let mut bytes = elf_bytes.clone();
        std::ptr::write(bytes.as_mut_ptr().cast::<Elf64Ehdr>(), header);
        bytes
    };

    let mut header = parsed_elf.file_header().clone();
    let initial_e_entry = header.e_entry;

    header.e_entry += 8;
    let elf_bytes = write_header(header.clone());
    let elf = ElfExecutable::load(&elf_bytes, loader.clone()).expect("validation failed");
    let executable: &Executable<TestContextObject> = &elf;
    assert_eq!(5, executable.get_entrypoint_instruction_offset());

    header.e_entry = 1;
    let elf_bytes = write_header(header.clone());
    assert!(matches!(
        ElfExecutable::load(&elf_bytes, loader.clone()),
        Err(ElfError::EntrypointOutOfBounds)
    ));

    header.e_entry = u64::MAX;
    let elf_bytes = write_header(header.clone());
    assert!(matches!(
        ElfExecutable::load(&elf_bytes, loader.clone()),
        Err(ElfError::EntrypointOutOfBounds)
    ));

    header.e_entry = initial_e_entry + ebpf::INSN_SIZE as u64 + 1;
    let elf_bytes = write_header(header.clone());
    assert!(matches!(
        ElfExecutable::load(&elf_bytes, loader.clone()),
        Err(ElfError::InvalidEntrypoint)
    ));

    header.e_entry = initial_e_entry;
    let elf_bytes = write_header(header);
    let elf = ElfExecutable::load(&elf_bytes, loader).expect("validation failed");
    let executable: &Executable<TestContextObject> = &elf;
    assert_eq!(4, executable.get_entrypoint_instruction_offset());
}

fn new_section(sh_addr: u64, sh_size: u64) -> Elf64Shdr {
    Elf64Shdr {
        sh_addr,
        sh_offset: sh_addr
            .checked_sub(ebpf::MM_RODATA_START)
            .unwrap_or(sh_addr),
        sh_size,
        sh_name: 0,
        sh_type: 0,
        sh_flags: 0,
        sh_link: 0,
        sh_info: 0,
        sh_addralign: 0,
        sh_entsize: 0,
    }
}

#[test]
fn test_owned_ro_sections_not_contiguous() {
    let config = Config::default();
    let elf_bytes = [0u8; 512];

    // there's a non-rodata section between two rodata sections
    let s1 = new_section(10, 10);
    let s2 = new_section(20, 10);
    let s3 = new_section(30, 10);

    let sections: [(Option<&[u8]>, &Elf64Shdr); 3] = [
        (Some(b".text"), &s1),
        (Some(b".dynamic"), &s2),
        (Some(b".rodata"), &s3),
    ];
    assert!(matches!(
        ElfExecutable::parse_ro_sections(
            &config,
            sections,
            &elf_bytes,
        ),
        Ok(Section::Owned(offset, data)) if offset == ebpf::MM_RODATA_START as usize + 10 && data.len() == 30
    ));
}

#[test]
fn test_owned_ro_sections_with_sh_offset() {
    let config = Config {
        reject_broken_elfs: false,
        ..Config::default()
    };
    let elf_bytes = [0u8; 512];

    // s2 is at a custom sh_offset. We need to merge into an owned buffer so
    // s2 can be moved to the right address offset.
    let s1 = new_section(10, 10);
    let mut s2 = new_section(20, 10);
    s2.sh_offset = 30;

    let sections: [(Option<&[u8]>, &Elf64Shdr); 2] =
        [(Some(b".text"), &s1), (Some(b".rodata"), &s2)];
    assert!(matches!(
        ElfExecutable::parse_ro_sections(
            &config,
            sections,
            &elf_bytes,
        ),
        Ok(Section::Owned(offset, data)) if offset == ebpf::MM_RODATA_START as usize + 10 && data.len() == 20
    ));
}

#[test]
fn test_sh_offset_not_same_as_vaddr() {
    let config = Config {
        reject_broken_elfs: true,
        enabled_sbpf_versions: SBPFVersion::V0..=SBPFVersion::V0,
        ..Config::default()
    };
    let elf_bytes = [0u8; 512];

    let mut s1 = new_section(10, 10);

    {
        let sections: [(Option<&[u8]>, &Elf64Shdr); 1] = [(Some(b".text"), &s1)];
        assert!(ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes).is_ok());
    }

    s1.sh_offset = 0;
    let sections: [(Option<&[u8]>, &Elf64Shdr); 1] = [(Some(b".text"), &s1)];
    assert_eq!(
        ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes),
        Err(ElfError::ValueOutOfBounds)
    );
}

#[test]
fn test_owned_ro_region_no_initial_gap() {
    let config = Config::default();
    let elf_bytes = [0u8; 512];

    // need an owned buffer so we can zero the address space taken by s2
    let s1 = new_section(0, 10);
    let s2 = new_section(10, 10);
    let s3 = new_section(20, 10);

    let sections: [(Option<&[u8]>, &Elf64Shdr); 3] = [
        (Some(b".text"), &s1),
        (Some(b".dynamic"), &s2),
        (Some(b".rodata"), &s3),
    ];
    let ro_section = ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes).unwrap();
    let ro_region = get_ro_region(&ro_section, &elf_bytes);
    let memory_mapping = MemoryMapping::new(vec![ro_region], &config, SBPFVersion::V0).unwrap();
    let owned_section = match &ro_section {
        Section::Owned(_offset, data) => data.as_slice(),
        _ => panic!(),
    };

    // [0..s3.sh_addr + s3.sh_size] is the valid ro memory area
    assert_eq!(
        memory_mapping
            .map(
                AccessType::Load,
                ebpf::MM_RODATA_START,
                s3.sh_addr + s3.sh_size
            )
            .unwrap(),
        owned_section.as_ptr() as u64,
    );

    // one byte past the ro section is not mappable
    memory_mapping
        .map(
            AccessType::Load,
            ebpf::MM_RODATA_START + s3.sh_addr + s3.sh_size,
            1,
        )
        .unwrap_err();
}

#[test]
fn test_owned_ro_region_initial_gap_mappable() {
    let config = Config {
        optimize_rodata: false,
        ..Config::default()
    };
    let elf_bytes = [0u8; 512];

    // the first section starts at a non-zero offset
    let s1 = new_section(10, 10);
    let s2 = new_section(20, 10);
    let s3 = new_section(30, 10);

    let sections: [(Option<&[u8]>, &Elf64Shdr); 3] = [
        (Some(b".text"), &s1),
        (Some(b".dynamic"), &s2),
        (Some(b".rodata"), &s3),
    ];
    // V2 requires optimize_rodata=true
    let ro_section = ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes).unwrap();
    let ro_region = get_ro_region(&ro_section, &elf_bytes);
    let memory_mapping = MemoryMapping::new(vec![ro_region], &config, SBPFVersion::V0).unwrap();
    let owned_section = match &ro_section {
        Section::Owned(_offset, data) => data.as_slice(),
        _ => panic!(),
    };

    // [s1.sh_addr..s3.sh_addr + s3.sh_size] is where the readonly data is.
    // But for backwards compatibility (config.optimize_rodata=false)
    // [0..s1.sh_addr] is mappable too (and zeroed).
    assert_eq!(
        memory_mapping
            .map(
                AccessType::Load,
                ebpf::MM_RODATA_START,
                s3.sh_addr + s3.sh_size
            )
            .unwrap(),
        owned_section.as_ptr() as u64,
    );

    // one byte past the ro section is not mappable
    memory_mapping
        .map(
            AccessType::Load,
            ebpf::MM_RODATA_START + s3.sh_addr + s3.sh_size,
            1,
        )
        .unwrap_err();
}

#[test]
fn test_owned_ro_region_initial_gap_map_error() {
    let config = Config::default();
    let elf_bytes = [0u8; 512];

    // the first section starts at a non-zero offset
    let s1 = new_section(10, 10);
    let s2 = new_section(20, 10);
    let s3 = new_section(30, 10);

    let sections: [(Option<&[u8]>, &Elf64Shdr); 3] = [
        (Some(b".text"), &s1),
        (Some(b".dynamic"), &s2),
        (Some(b".rodata"), &s3),
    ];
    let ro_section = ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes).unwrap();
    let owned_section = match &ro_section {
        Section::Owned(_offset, data) => data.as_slice(),
        _ => panic!(),
    };
    let ro_region = get_ro_region(&ro_section, &elf_bytes);
    let memory_mapping = MemoryMapping::new(vec![ro_region], &config, SBPFVersion::V0).unwrap();

    // s1 starts at sh_addr=10 so [MM_RODATA_START..MM_RODATA_START + 10] is not mappable

    // the low bound of the initial gap is not mappable
    memory_mapping
        .map(AccessType::Load, ebpf::MM_RODATA_START, 1)
        .unwrap_err();

    // the hi bound of the initial gap is not mappable
    memory_mapping
        .map(AccessType::Load, ebpf::MM_RODATA_START + s1.sh_addr - 1, 1)
        .unwrap_err();

    // [s1.sh_addr..s3.sh_addr + s3.sh_size] is the valid ro memory area
    assert_eq!(
        memory_mapping
            .map(
                AccessType::Load,
                ebpf::MM_RODATA_START + s1.sh_addr,
                s3.sh_addr + s3.sh_size - s1.sh_addr
            )
            .unwrap(),
        owned_section.as_ptr() as u64,
    );

    // one byte past the ro section is not mappable
    memory_mapping
        .map(
            AccessType::Load,
            ebpf::MM_RODATA_START + s3.sh_addr + s3.sh_size,
            1,
        )
        .unwrap_err();
}

#[test]
fn test_borrowed_ro_sections_disabled() {
    let config = Config {
        optimize_rodata: false,
        ..Config::default()
    };
    let elf_bytes = [0u8; 512];

    // s1 and s2 are contiguous, the rodata section can be borrowed from the
    // original elf input but config.borrow_rodata=false
    let s1 = new_section(0, 10);
    let s2 = new_section(10, 10);

    let sections: [(Option<&[u8]>, &Elf64Shdr); 2] =
        [(Some(b".text"), &s1), (Some(b".rodata"), &s2)];
    assert!(matches!(
        ElfExecutable::parse_ro_sections(
            &config,
            sections,
            &elf_bytes,
        ),
        Ok(Section::Owned(offset, data)) if offset == ebpf::MM_RODATA_START as usize && data.len() == 20
    ));
}

#[test]
fn test_borrowed_ro_sections() {
    let config = Config::default();
    let elf_bytes = [0u8; 512];
    let s1 = new_section(0, 10);
    let s2 = new_section(20, 10);
    let s3 = new_section(40, 10);
    let s4 = new_section(50, 10);
    let sections: [(Option<&[u8]>, &Elf64Shdr); 4] = [
        (Some(b".dynsym"), &s1),
        (Some(b".text"), &s2),
        (Some(b".rodata"), &s3),
        (Some(b".dynamic"), &s4),
    ];
    assert_eq!(
        ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes),
        Ok(Section::Borrowed(
            ebpf::MM_RODATA_START as usize + 20,
            20..50
        ))
    );
}

#[test]
fn test_borrowed_ro_region_no_initial_gap() {
    let config = Config::default();
    let elf_bytes = [0u8; 512];
    let s1 = new_section(0, 10);
    let s2 = new_section(10, 10);
    let s3 = new_section(20, 10);
    let sections: [(Option<&[u8]>, &Elf64Shdr); 3] = [
        (Some(b".text"), &s1),
        (Some(b".rodata"), &s2),
        (Some(b".dynamic"), &s3),
    ];
    let ro_section = ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes).unwrap();
    let ro_region = get_ro_region(&ro_section, &elf_bytes);
    let memory_mapping = MemoryMapping::new(vec![ro_region], &config, SBPFVersion::V0).unwrap();

    // s1 starts at sh_offset=0 so [0..s2.sh_offset + s2.sh_size]
    // is the valid ro memory area
    assert_eq!(
        memory_mapping
            .map(
                AccessType::Load,
                ebpf::MM_RODATA_START + s1.sh_offset,
                s2.sh_offset + s2.sh_size
            )
            .unwrap(),
        elf_bytes.as_ptr() as u64,
    );

    // one byte past the ro section is not mappable
    memory_mapping
        .map(AccessType::Load, ebpf::MM_RODATA_START + s3.sh_offset, 1)
        .unwrap_err();
}

#[test]
fn test_borrowed_ro_region_initial_gap() {
    let config = Config::default();
    let elf_bytes = [0u8; 512];
    let s1 = new_section(0, 10);
    let s2 = new_section(10, 10);
    let s3 = new_section(20, 10);
    let sections: [(Option<&[u8]>, &Elf64Shdr); 3] = [
        (Some(b".dynamic"), &s1),
        (Some(b".text"), &s2),
        (Some(b".rodata"), &s3),
    ];
    let ro_section = ElfExecutable::parse_ro_sections(&config, sections, &elf_bytes).unwrap();
    let ro_region = get_ro_region(&ro_section, &elf_bytes);
    let memory_mapping = MemoryMapping::new(vec![ro_region], &config, SBPFVersion::V0).unwrap();

    // s2 starts at sh_addr=10 so [0..10] is not mappable

    // the low bound of the initial gap is not mappable
    memory_mapping
        .map(AccessType::Load, ebpf::MM_RODATA_START + s1.sh_offset, 1)
        .unwrap_err();

    // the hi bound of the initial gap is not mappable
    memory_mapping
        .map(
            AccessType::Load,
            ebpf::MM_RODATA_START + s2.sh_offset - 1,
            1,
        )
        .unwrap_err();

    // [s2.sh_offset..s3.sh_offset + s3.sh_size] is the valid ro memory area
    assert_eq!(
        memory_mapping
            .map(
                AccessType::Load,
                ebpf::MM_RODATA_START + s2.sh_offset,
                s3.sh_offset + s3.sh_size - s2.sh_offset
            )
            .unwrap(),
        elf_bytes[s2.sh_offset as usize..].as_ptr() as u64,
    );

    // one byte past the ro section is not mappable
    memory_mapping
        .map(
            AccessType::Load,
            ebpf::MM_RODATA_START + s3.sh_offset + s3.sh_size,
            1,
        )
        .unwrap_err();
}

#[test]
#[should_panic(expected = r#"validation failed: WritableSectionNotSupported(".data")"#)]
fn test_writable_data_section() {
    let elf_bytes =
        std::fs::read("tests/elfs/data_section_sbpfv0.so").expect("failed to read elf file");
    ElfExecutable::load(&elf_bytes, loader()).expect("validation failed");
}

#[test]
#[should_panic(expected = r#"validation failed: WritableSectionNotSupported(".bss")"#)]
fn test_bss_section() {
    let elf_bytes =
        std::fs::read("tests/elfs/bss_section_sbpfv0.so").expect("failed to read elf file");
    ElfExecutable::load(&elf_bytes, loader()).expect("validation failed");
}

#[test]
#[should_panic(expected = "validation failed: InvalidProgramHeader")]
fn test_program_headers_overflow() {
    let elf_bytes =
        std::fs::read("tests/elfs/program_headers_overflow.so").expect("failed to read elf file");
    ElfExecutable::load(&elf_bytes, loader()).expect("validation failed");
}

#[test]
#[should_panic(expected = "validation failed: RelativeJumpOutOfBounds(8)")]
fn test_relative_call_oob_backward() {
    let mut elf_bytes =
        std::fs::read("tests/elfs/relative_call_sbpfv0.so").expect("failed to read elf file");
    LittleEndian::write_i32(&mut elf_bytes[0x164..0x168], -11i32);
    ElfExecutable::load(&elf_bytes, loader()).expect("validation failed");
}

#[test]
#[should_panic(expected = "validation failed: RelativeJumpOutOfBounds(11)")]
fn test_relative_call_oob_forward() {
    let mut elf_bytes =
        std::fs::read("tests/elfs/relative_call_sbpfv0.so").expect("failed to read elf file");
    LittleEndian::write_i32(&mut elf_bytes[0x17c..0x180], 5);
    ElfExecutable::load(&elf_bytes, loader()).expect("validation failed");
}

#[test]
#[should_panic(expected = "validation failed: UnresolvedSymbol(\"log\", 39, 312)")]
fn test_err_unresolved_syscall_reloc_64_32() {
    let loader = BuiltinProgram::new_loader(Config {
        enabled_sbpf_versions: SBPFVersion::V0..=SBPFVersion::V0,
        reject_broken_elfs: true,
        ..Config::default()
    });
    let elf_bytes =
        std::fs::read("tests/elfs/syscall_reloc_64_32_sbpfv0.so").expect("failed to read elf file");
    ElfExecutable::load(&elf_bytes, Arc::new(loader)).expect("validation failed");
}

#[test]
fn test_long_section_name() {
    let elf_bytes = std::fs::read("tests/elfs/long_section_name.so").unwrap();
    assert_error!(
        Elf64::parse(&elf_bytes),
        "StringTooLong({:?}, {})",
        ".bss.__rust_no_alloc_shim_is_unstable"
            .get(0..SECTION_NAME_LENGTH_MAXIMUM)
            .unwrap(),
        SECTION_NAME_LENGTH_MAXIMUM
    );
}

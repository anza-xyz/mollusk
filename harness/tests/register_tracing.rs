#[cfg(feature = "register-tracing")]
#[test]
fn test_custom_register_tracing_callback() {
    use {
        mollusk_svm::{InvocationInspectCallback, Mollusk},
        solana_account::Account,
        solana_instruction::{AccountMeta, Instruction},
        solana_program_runtime::invoke_context::{Executable, InvokeContext, RegisterTrace},
        solana_pubkey::Pubkey,
        solana_transaction_context::{InstructionAccount, InstructionContext},
        std::{cell::RefCell, collections::HashMap, rc::Rc},
    };

    struct TracingData {
        program_id: Pubkey,
        executed_jump_instructions_count: usize,
    }

    struct CustomRegisterTracingCallback {
        tracing_data: Rc<RefCell<HashMap<Pubkey, TracingData>>>,
    }

    impl CustomRegisterTracingCallback {
        fn handler(
            &self,
            instruction_context: InstructionContext,
            executable: &Executable,
            register_trace: RegisterTrace,
        ) -> Result<(), Box<dyn std::error::Error + 'static>> {
            let mut tracing_data = self.tracing_data.try_borrow_mut()?;

            let program_id = instruction_context.get_program_key().unwrap();
            let (_vm_addr, program) = executable.get_text_bytes();
            let executed_jump_instructions_count = register_trace
                .iter()
                .map(|registers| {
                    (
                        registers,
                        solana_program_runtime::solana_sbpf::ebpf::get_insn_unchecked(
                            program,
                            registers[11] as usize,
                        ),
                    )
                })
                .filter(|(_registers, insn)| {
                    insn.opc & 7 == solana_program_runtime::solana_sbpf::ebpf::BPF_JMP
                        && insn.opc != solana_program_runtime::solana_sbpf::ebpf::BPF_JA
                })
                .count();
            let entry = tracing_data.entry(*program_id).or_insert(TracingData {
                program_id: *program_id,
                executed_jump_instructions_count: 0,
            });
            entry.executed_jump_instructions_count = entry
                .executed_jump_instructions_count
                .saturating_add(executed_jump_instructions_count);

            Ok(())
        }
    }

    impl InvocationInspectCallback for CustomRegisterTracingCallback {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }

        fn before_invocation(
            &self,
            _: &Mollusk,
            _: &Pubkey,
            _: &[u8],
            _: &[InstructionAccount],
            _: &mut InvokeContext,
            _register_tracing_enabled: bool,
        ) {
        }

        fn after_invocation(
            &self,
            _: &Mollusk,
            invoke_context: &InvokeContext,
            register_tracing_enabled: bool,
        ) {
            // Only process traces if register tracing was enabled.
            if register_tracing_enabled {
                invoke_context.iterate_vm_traces(
                    &|instruction_context: InstructionContext,
                      executable: &Executable,
                      register_trace: RegisterTrace| {
                        if let Err(e) =
                            self.handler(instruction_context, executable, register_trace)
                        {
                            eprintln!("Error collecting the register tracing: {}", e);
                        }
                    },
                );
            }
        }
    }

    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let payer_pk = Pubkey::new_unique();
    // Use new_debuggable with register tracing enabled.
    let mut mollusk = Mollusk::new_debuggable(
        &program_id,
        "test_program_primary",
        /* enable_register_tracing */ true,
    );

    // Phase 1 - basic register tracing test.

    // Have a custom register tracing handler counting the total number of executed
    // jump instructions per program_id.
    let tracing_data = Rc::new(RefCell::new(HashMap::<Pubkey, TracingData>::new()));
    mollusk.invocation_inspect_callback = Box::new(CustomRegisterTracingCallback {
        tracing_data: Rc::clone(&tracing_data),
    });

    let (system_program_id, system_account) =
        mollusk_svm::program::keyed_account_for_system_program();

    let ix_data = [0, 0];
    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_data,
        vec![
            AccountMeta::new(payer_pk, true),
            AccountMeta::new(system_program_id, false),
        ],
    );

    let base_lamports = 100_000_000u64;
    let accounts = vec![
        (payer_pk, Account::new(base_lamports, 0, &system_program_id)),
        (system_program_id, system_account),
    ];

    // Execute the instruction.
    let _ = mollusk.process_instruction(&instruction, &accounts);

    let executed_jump_instruction_count_from_phase1;
    // Let's check the outcome of the custom register tracing callback.
    {
        assert_eq!(tracing_data.borrow().len(), 1);
        let td = tracing_data.borrow();
        let collected_data = td.get(&program_id).unwrap();

        // Check it's the program_id only on our list.
        assert_eq!(collected_data.program_id, program_id);
        // Check the number of executed jump class instructions is greater than 0.
        assert!(collected_data.executed_jump_instructions_count > 0);

        // Store this value for a later comparison.
        executed_jump_instruction_count_from_phase1 =
            collected_data.executed_jump_instructions_count;
    }

    // Phase 2 - check that register tracing is disabled when constructing
    // Mollusk with enable_register_tracing=false.
    {
        // Clear the tracing data collected so far.
        {
            let mut td = tracing_data.borrow_mut();
            td.clear();
        }

        // Create a new Mollusk instance with register tracing disabled.
        let mut mollusk_no_tracing = Mollusk::new_debuggable(
            &program_id,
            "test_program_primary",
            /* enable_register_tracing */ false,
        );
        mollusk_no_tracing.invocation_inspect_callback = Box::new(CustomRegisterTracingCallback {
            tracing_data: Rc::clone(&tracing_data),
        });

        // Execute the same instruction again.
        let _ = mollusk_no_tracing.process_instruction(&instruction, &accounts);

        let td = tracing_data.borrow();
        // We expect it to be empty since tracing was disabled!
        assert!(td.is_empty());
    }

    // Phase 3 - check we can have register tracing enabled for a new instance of
    // Mollusk.
    {
        // Create a new Mollusk instance with register tracing enabled.
        let mut mollusk_with_tracing = Mollusk::new_debuggable(
            &program_id,
            "test_program_primary",
            /* enable_register_tracing */ true,
        );
        mollusk_with_tracing.invocation_inspect_callback =
            Box::new(CustomRegisterTracingCallback {
                tracing_data: Rc::clone(&tracing_data),
            });

        // Execute the same instruction again.
        let _ = mollusk_with_tracing.process_instruction(&instruction, &accounts);

        let td = tracing_data.borrow();
        let collected_data = td.get(&program_id).unwrap();

        // Check again it's the program_id only on our list.
        assert_eq!(collected_data.program_id, program_id);
        // Check the number of executed jump instructions is the same as we did in
        // phase 1 of this test.
        assert!(
            collected_data.executed_jump_instructions_count
                == executed_jump_instruction_count_from_phase1
        );
    }
}

#[cfg(feature = "sbpf-debugger")]
#[cfg(test)]
mod debugger_tests {
    use {
        mollusk_svm::{
            program::create_program_account_loader_v3,
            register_tracing::{compute_hash, DefaultRegisterTracingCallback},
            Mollusk,
        },
        solana_account::Account,
        solana_message::{AccountMeta, Instruction},
        solana_pubkey::Pubkey,
        std::{
            collections::HashMap,
            io::{BufRead, BufReader, Write},
            net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs},
            time::Duration,
        },
    };

    fn read_reply<R: BufRead>(reader: &mut R) -> std::io::Result<String> {
        let mut buf = Vec::new();

        // Read till the # character.
        reader.read_until(b'#', &mut buf)?;
        // Then read exactly 2 bytes representing the checksum.
        let mut cbuf = [0];
        let _ = reader.read(&mut cbuf)?;
        let _ = buf.write(&cbuf)?;
        let _ = reader.read(&mut cbuf)?;
        let _ = buf.write(&cbuf)?;
        let reply = String::from_utf8_lossy(&buf).to_string();
        // eprintln!("gdbstub reply: {}", reply);
        Ok(reply)
    }

    #[allow(unused)]
    fn gdb_read_memory_cmd(addr: u64, size: usize) -> Vec<u8> {
        let payload = format!("m{:x},{:x}", addr, size);
        let checksum: u8 = payload.bytes().fold(0u8, |acc, b| acc.wrapping_add(b));
        format!("${}#{:02x}", payload, checksum).into_bytes()
    }

    #[allow(unused)]
    fn gdb_parse_packet(input: &str) -> Option<Vec<u8>> {
        const GDB_RLE_OFFSET: u8 = 29;

        let data = input.strip_prefix('+').unwrap_or(input);
        let data = data.strip_prefix('$')?;
        // might be a console output $O..
        let data = data.strip_prefix('O').unwrap_or(data);
        let data = data.split('#').next()?;

        let mut hex_str = String::new();
        let mut chars = data.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '*' {
                let count_char = chars.next()?;
                let repeat = (count_char as u8).saturating_sub(GDB_RLE_OFFSET) as usize;
                let last = hex_str.chars().last()?;
                for _ in 0..repeat {
                    hex_str.push(last);
                }
            } else {
                hex_str.push(c);
            }
        }

        hex::decode(&hex_str).ok()
    }

    fn gdb_read_register_cmd(reg_num: usize) -> Vec<u8> {
        let payload = format!("p{reg_num:x}");
        let checksum: u8 = payload.bytes().fold(0u8, |acc, b| acc.wrapping_add(b));
        format!("${}#{:02x}", payload, checksum).into_bytes()
    }

    #[allow(unused)]
    fn stub_read_memory_chunked<R: BufRead, W: Write>(
        writer: &mut W,
        reader: &mut R,
        addr: u64,
        total_size: usize,
        chunk_size: usize,
    ) -> std::io::Result<Vec<u8>> {
        let mut result = Vec::with_capacity(total_size);
        let mut offset = 0;

        while offset < total_size {
            let size = std::cmp::min(chunk_size, total_size - offset);
            writer.write_all(&gdb_read_memory_cmd(addr + offset as u64, size))?;
            writer.flush()?;

            let reply = read_reply(reader)?;
            let parsed_reply = gdb_parse_packet(&reply).ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid data",
            ))?;
            result.extend_from_slice(&parsed_reply);

            offset += size;
        }

        Ok(result)
    }

    fn stub_read_register<R: BufRead, W: Write>(
        writer: &mut W,
        reader: &mut R,
        reg_num: usize,
    ) -> std::io::Result<u64> {
        let cmd = gdb_read_register_cmd(reg_num);
        writer.write_all(&cmd)?;
        writer.flush()?;
        let reply = read_reply(reader)?;
        let parsed_reply =
            gdb_parse_packet(&reply).ok_or(std::io::Error::other("invalid packet"))?;
        let data = parsed_reply
            .get(..8)
            .and_then(|s| s.try_into().ok())
            .ok_or(std::io::Error::other("expected 8 bytes"))?;
        let reg_value = u64::from_le_bytes(data);
        Ok(reg_value)
    }

    fn stub_read_all_registers<R: BufRead, W: Write>(
        writer: &mut W,
        reader: &mut R,
    ) -> std::io::Result<HashMap<String, u64>> {
        let mut map = HashMap::new();
        for reg_num in 0..=9 {
            map.insert(
                format!("r{}", reg_num),
                stub_read_register(writer, reader, reg_num)?,
            );
        }
        map.insert("fp".to_string(), stub_read_register(writer, reader, 10)?);
        map.insert("pc".to_string(), stub_read_register(writer, reader, 11)?);
        map.insert(
            "icount_remain".to_string(),
            stub_read_register(writer, reader, 12)?,
        );
        Ok(map)
    }

    fn stub_fetch_debug_metadata<R: BufRead, W: Write>(
        mut reader: &mut R,
        writer: &mut W,
    ) -> Result<HashMap<String, String>, std::io::Error> {
        // Take advantage of the metadata monitor command in sbpf
        // to check what the runtime is already passing to us.
        // (lldb) process plugin packet monitor metadata
        // "metadata" -> 6d65746164617461
        writer.write_all(b"$qRcmd,6d65746164617461#9d")?;
        let reply = read_reply(&mut reader)?;
        let parsed = gdb_parse_packet(&reply).ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Can't parse metadata monitor command reply",
        ))?;

        // Drain the OK following.
        let reply = read_reply(&mut reader)?;
        assert_eq!("$OK#9a", reply);

        // We expect a plain text metadata with a newline appended so have it trimmed.
        let parsed = String::from_utf8_lossy(&parsed).trim_end().to_string();
        let parsed_map: HashMap<_, _> = parsed
            .split(';')
            .filter_map(|e| e.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Ok(parsed_map)
    }

    fn stub_send_continue_command<R: BufRead, W: Write>(
        mut reader: &mut R,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        writer.write_all(b"$vCont;c:p1.-1#0f")?;
        let reply = read_reply(&mut reader)?;
        assert_eq!("+$W00#b7", reply);
        Ok(())
    }

    fn stub_connect<A: ToSocketAddrs>(
        stub_addr: A,
        mut retries: usize,
    ) -> Result<(BufReader<TcpStream>, TcpStream), std::io::Error> {
        let (reader, writer) = loop {
            retries -= 1;
            match std::net::TcpStream::connect(&stub_addr) {
                Err(e) => {
                    if retries == 0 {
                        return Err(e);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Ok(stream) => break (BufReader::new(stream.try_clone()?), stream),
            }
        };
        Ok((reader, writer))
    }

    #[test]
    fn test_debugger() {
        const SBF_DEBUG_PORT: u16 = 21212;
        const STUB_ADDR: SocketAddr =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), SBF_DEBUG_PORT);
        const STUB_CONNECT_RETRIES: usize = 30;

        std::env::set_var("SBF_OUT_DIR", "../target/deploy");
        std::env::set_var("SBF_DEBUG_PORT", SBF_DEBUG_PORT.to_string());

        let program_id = Pubkey::new_unique();
        let cpi_target_program_id = Pubkey::new_unique();
        // Use new_debuggable with register tracing enabled.
        let mut mollusk = Mollusk::new_debuggable(
            &program_id,
            "test_program_primary",
            /* enable_register_tracing */ true,
        );

        mollusk.add_program_with_loader(
            &cpi_target_program_id,
            "test_program_cpi_target",
            &mollusk_svm::program::loader_keys::LOADER_V3,
        );
        mollusk.feature_set.activate(
            &agave_feature_set::provide_instruction_data_offset_in_vm_r2::id(),
            0,
        );

        // Phase 1 - test that debugger won't start (won't hang listening)
        // as we filter by *inexisting* program_id.

        let tracing_callback: &mut DefaultRegisterTracingCallback = mollusk
            .invocation_inspect_callback
            .as_any_mut()
            .downcast_mut()
            .unwrap();
        tracing_callback.sbf_trace_filter = format!("program_id == {}", Pubkey::new_unique());

        let data = &[1, 2, 3, 4, 5];
        let space = data.len();
        let lamports = mollusk.sysvars.rent.minimum_balance(space);

        let key = Pubkey::new_unique();
        let account = Account::new(lamports, space, &cpi_target_program_id);
        let (instruction, instruction_data_len) = {
            let mut instruction_data = vec![4];
            instruction_data.extend_from_slice(cpi_target_program_id.as_ref());
            instruction_data.extend_from_slice(data);
            (
                Instruction::new_with_bytes(
                    program_id,
                    &instruction_data,
                    vec![
                        AccountMeta::new(key, true),
                        AccountMeta::new_readonly(cpi_target_program_id, false),
                    ],
                ),
                instruction_data.len(),
            )
        };

        let accounts = &[
            (key, account.clone()),
            (
                cpi_target_program_id,
                create_program_account_loader_v3(&cpi_target_program_id),
            ),
        ];

        // Execute the instruction. No TCP listening hang expected.
        let _ = mollusk.process_instruction(&instruction, accounts);

        // Phase 2 - test that debugger *will* start
        // - if the filter is empty (this is the case in this test since the debug_port
        //   is set)
        // - if the filter matches either CPI callee or CPI caller
        // - if the filter is for the CPI target
        let empty_filter = "";
        let filters = [
            empty_filter.into(),
            format!(
                "program_id == {} || program_id == {}",
                program_id, cpi_target_program_id
            ),
            format!("program_id == {}", cpi_target_program_id),
        ];
        for filter in filters.into_iter() {
            let tracing_callback: &mut DefaultRegisterTracingCallback = mollusk
                .invocation_inspect_callback
                .as_any_mut()
                .downcast_mut()
                .unwrap();
            tracing_callback.sbf_trace_filter = filter;

            let program_id_file = std::path::PathBuf::from(&tracing_callback.sbf_trace_dir)
                .join("program_ids")
                .with_extension("map");

            // This is the expected program IDs <-> SHA-256 mapping.
            let expected_program_ids = format!(
                "{}={}\n{}={}\n",
                program_id,
                compute_hash(
                    mollusk
                        .program_cache
                        .get_program_elf_bytes(&program_id)
                        .unwrap()
                        .as_slice()
                ),
                cpi_target_program_id,
                compute_hash(
                    mollusk
                        .program_cache
                        .get_program_elf_bytes(&cpi_target_program_id)
                        .unwrap()
                        .as_slice()
                )
            );

            // Execute the instruction.
            std::thread::scope(|s| {
                let client_jh = s.spawn(|| -> Result<(), std::io::Error> {
                    // Connect to the debugger stub.
                    let (mut reader, mut writer) = stub_connect(STUB_ADDR, STUB_CONNECT_RETRIES)?;

                    // Check r2 - it should point to the instruction data whereas the length is 8
                    // bytes prior to it.
                    let registers = stub_read_all_registers(&mut writer, &mut reader)?;
                    let data_addr = registers["r2"];
                    let data_len_addr = data_addr - 8;
                    let data_len =
                        stub_read_memory_chunked(&mut writer, &mut reader, data_len_addr, 8, 1024)?
                            .try_into()
                            .map_err(|_| std::io::Error::other("expected 8 bytes"))?;
                    let data_len = u64::from_le_bytes(data_len) as usize;
                    assert!(instruction_data_len == data_len);
                    let data = stub_read_memory_chunked(
                        &mut writer,
                        &mut reader,
                        data_addr,
                        data_len,
                        1024,
                    )?;
                    assert!(instruction.data == data);

                    // Don't use this approach as it depends on the ABI.
                    // // Verify the program_id reported by the gdbstub matches the one we're
                    // // debugging.
                    // let mut reply = stub_read_memory_chunked(
                    //     &mut writer,
                    //     &mut reader,
                    //     0x400000000,     // The input buffer of the program starts from here.
                    //     1 * 1024 * 1024, // Read 1MB just in case.
                    //     1024,            // Read in chunks of 1024 bytes.
                    // )?;
                    // let (deserialized_program_id, _, _) =
                    //     unsafe { solana_program_entrypoint::deserialize(reply.as_mut_ptr()) };
                    // assert_eq!(program_id, *deserialized_program_id);
                    let parsed_map = stub_fetch_debug_metadata(&mut reader, &mut writer)?;

                    // After parsing the reply check the runtime has passed to us the
                    // expected program_id in the metadata.
                    assert!(
                        parsed_map.get("program_id") == Some(&program_id.to_string())
                            && parsed_map.get("cpi_level") == Some(&"0".to_string())
                            && parsed_map.get("caller") == Some(&"none".to_string())
                    );

                    // Fire the CPI handling prior to issuing the continue command.
                    let cpi_client_jh = s.spawn(|| -> Result<(), std::io::Error> {
                        // The CPI means we have another gdb stub instantiated and listening.
                        let (mut reader, mut writer) =
                            stub_connect(STUB_ADDR, STUB_CONNECT_RETRIES)?;

                        let parsed_map = stub_fetch_debug_metadata(&mut reader, &mut writer)?;

                        // Check the CPI callee and caller and level.
                        assert!(
                            parsed_map.get("program_id")
                                == Some(&cpi_target_program_id.to_string())
                                && parsed_map.get("cpi_level") == Some(&"1".to_string())
                                && parsed_map.get("caller") == Some(&program_id.to_string())
                        );

                        // Issue the continue command.
                        stub_send_continue_command(&mut reader, &mut writer)?;

                        Ok(())
                    });

                    // Issue the continue command.
                    stub_send_continue_command(&mut reader, &mut writer)?;

                    cpi_client_jh.join().unwrap().expect("cpi client error");

                    Ok(())
                });

                // Processing...
                let _ = mollusk.process_instruction(&instruction, accounts);

                client_jh.join().unwrap().expect("client error");
            });

            // Phase 3.
            // Check the program_ids <-> elf sha256 mapping table.
            let read_program_ids = std::fs::read_to_string(&program_id_file).unwrap();
            assert_eq!(read_program_ids, expected_program_ids);
        }
    }
}

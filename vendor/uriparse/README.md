# uriparse-rs

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://travis-ci.org/sgodwincs/uriparse-rs.svg?branch=master)](https://travis-ci.org/sgodwincs/uriparse-rs)
[![Crates.io Version](https://img.shields.io/crates/v/uriparse.svg)](https://crates.io/crates/uriparse)

Implementation of [RFC3986](https://tools.ietf.org/html/rfc3986) including URIs and URI references.

[Documentation](https://docs.rs/uriparse/)

## Goals

The goal of this crate is to provide types for efficiently creating, modifying, and parsing URIs, relative references, and URI references.

Each of the three types are described below.

### What is a `URI`?

URI stands for Uniform Resource Indentifier and serves to identify some resource. OK, that probably doesn't explain much, so let's look at some examples:

 - https://www.google.com/
 - ftps://example.com/help?q=example
 - urn:resource
 - http://mywebsite:8080#tag

As you can see, a URI is composed of individual parts, specifically: scheme, authority, path, query, and fragment.

#### Scheme

The scheme is the portion before the first `':'`, such as `"https"` or `"urn"` and determines the specification that is used for the rest of the URI. Each scheme may have varying restrictions on the form of the URI.

For example, the URN (Uniform Resource Name) scheme does not use `'/'` at all, whereas HTTP obviously does. This crate supports the most generic syntax that is valid, that is, a URI such as `"urn:/test/"` may be a valid URI, but it is not a valid URN URI.

There are a lot of registered schemes that can be seen at [here](https://www.iana.org/assignments/uri-schemes/uri-schemes.xhtml), and this crate provides an easy way of accessing them with the `Scheme` type (e.g. `Scheme::HTTPS`). You can also use schemes that are not registered.

#### Authority

The authority is always preceded by `"//"` and is everything up to a `'/'`, `'?'`, `'#'`, or until the end of the URI. It's composed of three parts: user information, host, and port.

The user information allows for providing a username and password (e.g. `"http://username:password@127.0.0.1/"`) for the resource. However, using a password is deprecated by the specification, so you should avoid it if you can as lots of servers log URIs.

The host provides the name of the machine that hosts the resource. It is either an IPv4 address, an IPv6 address, or a registered name (e.g. a domain name).

Lastly is the port for which to one is to establish a connection to the machine, presumably using TCP or UDP. It is not required to be specified and many schemes specify default ports (e.g. HTTP uses port 80 by default).

#### Path

The path is the main part of a URI and is always there even if it does not appear to be so. For example, the path for the URI `"http://example.com"` is `"/"` and the path for the URI `"urn:"` is `""`. A path following an authority *always* starts with a preceding `'/'`.

Looking at a URI, the path starts from the authority (if there is one, otherwise the scheme) and continues until a `'?'`, `'#'`, or the end of the URI. The meaning of the contents of the query are highly dependent on the scheme being used.

There are quite a few different classifications that paths can have, and I recommend reading over the corresponding section in the specification for more information.

#### Query

The query is everything after the first `'?'` until a `'#'` or the end of the URI. Like the path, the meaning of the contents of the query are highly dependent on the scheme being used.

#### Fragment

The fragment is everything after the first `'#'`. It serves to identify a secondary resource and is dependent on the media type of the primary resource being described. Due to this, it's description is often orthogonal to the scheme's specification.

### What is a `RelativeReference`?

A relative reference is very similar to a URI with one key distinction: it does not have a scheme. Everything else, however, is identical. Relative references are often used where the base URI for a resource is implicitly defined.

### What is a `URIReference`?

A URI reference is simply the union of all valid URIs and relative references. It can represent references that may or may not have schemes specified.

## Performance

As stated in the goals section, this crate aims to be able to efficiently parse URIs. This primarily means minimal allocations. Currently, the only allocation performed is when parsing the path into a `Vec<Segment<'segment>>`, although the segments themselves are still references to the original source.

## Normalization

This crate, by default, does not perform any normalization. However, the user can normalize components after parsing either individually or all at once using the provided `normalize` functions on the corresponding components. The normalizations provided are case normalization, percent-encoding normalization, and path segment normalization.

However, when comparing components, percent-encoding normalization is taken into consideration. For example, if you were to compare the two paths `/my/path` and `/my/p%61th`, they would be considered equal by this crate. It is done similarly with respect to hashing as well.

## Query String

This crate does not do query string parsing, it will simply make sure that it is a valid query string as defined by [[RFC3986, Section 3.4]](https://tools.ietf.org/html/rfc3986#section-3.4). You will need to use another crate (e.g. [queryst](https://github.com/rustless/queryst)) if you want it parsed.

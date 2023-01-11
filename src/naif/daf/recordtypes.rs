/*
 * ANISE Toolkit
 * Copyright (C) 2021-2022 Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Documentation: https://nyxspace.com/
 */

use zerocopy::{AsBytes, FromBytes};

use crate::{naif::Endian, prelude::AniseError, DBL_SIZE};
use log::{error, warn};

use super::{NAIFRecord, RCRD_LEN};

#[derive(Debug, Clone, FromBytes, AsBytes)]
#[repr(C)]
pub struct DAFFileRecord {
    pub locidw: [u8; 8],
    pub nd: u32,
    pub ni: u32,
    pub locifn: [u8; 60],
    pub forward: u32,
    pub backward: u32,
    pub free_addr: u32,
    pub locfmt: [u8; 8],
    pub prenul: [u8; 603],
    pub ftpstr: [u8; 28],
    pub pstnul: [u8; 297],
}

impl Default for DAFFileRecord {
    fn default() -> Self {
        Self {
            locidw: [0; 8],
            nd: Default::default(),
            ni: Default::default(),
            locifn: [0; 60],
            forward: Default::default(),
            backward: Default::default(),
            free_addr: Default::default(),
            locfmt: [0; 8],
            prenul: [0; 603],
            ftpstr: [0; 28],
            pstnul: [0; 297],
        }
    }
}

impl NAIFRecord for DAFFileRecord {}

impl DAFFileRecord {
    pub fn ni(&self) -> usize {
        self.ni as usize
    }

    pub fn nd(&self) -> usize {
        self.nd as usize
    }

    pub fn fwrd_idx(&self) -> usize {
        self.forward as usize
    }

    pub fn summary_size(&self) -> usize {
        (self.nd + (self.ni + 1) / 2) as usize
    }

    pub fn identification(&self) -> Result<&str, AniseError> {
        let str_locidw = core::str::from_utf8(&self.locidw).map_err(|_| {
            AniseError::DAFParserError("Could not parse identification string".to_owned())
        })?;

        if &str_locidw[0..3] != "DAF" || str_locidw.chars().nth(3) != Some('/') {
            Err(AniseError::DAFParserError(format!(
                "Cannot parse file whose identifier is not DAF: `{}`",
                str_locidw,
            )))
        } else {
            match str_locidw[4..].trim() {
                "SPK" => Ok("SPK"),
                "PCK" => Ok("PCK"),
                _ => {
                    error!("DAF of type `{}` is not yet supported", &str_locidw[4..]);
                    Err(AniseError::DAFParserError(format!(
                        "Cannot parse SPICE data of type `{}`",
                        str_locidw
                    )))
                }
            }
        }
    }

    pub fn endianness(&self) -> Result<Endian, AniseError> {
        let str_endianness = core::str::from_utf8(&self.locfmt)
            .map_err(|_| AniseError::DAFParserError("Could not parse endianness".to_owned()))?;

        let file_endian = if str_endianness == "LTL-IEEE" {
            Endian::Little
        } else if str_endianness == "BIG-IEEE" {
            Endian::Big
        } else {
            return Err(AniseError::DAFParserError(format!(
                "Could not understand endianness: `{}`",
                str_endianness
            )));
        };
        if file_endian != Endian::f64_native() || file_endian != Endian::u64_native() {
            Err(AniseError::DAFParserError(
                "Input file has different endian-ness than the platform and cannot be decoded"
                    .to_string(),
            ))
        } else {
            Ok(file_endian)
        }
    }

    pub fn internal_filename(&self) -> Result<&str, AniseError> {
        match core::str::from_utf8(&self.locifn) {
            Ok(filename) => Ok(filename.trim()),
            Err(e) => Err(AniseError::DAFParserError(format!("{e}"))),
        }
    }
}

#[derive(AsBytes, Clone, Copy, Debug, Default, FromBytes)]
#[repr(C)]
pub struct DAFSummaryRecord {
    next_record: f64,
    prev_record: f64,
    num_summaries: f64,
}

impl NAIFRecord for DAFSummaryRecord {}

impl DAFSummaryRecord {
    pub fn next_record(&self) -> usize {
        self.next_record as usize
    }

    pub fn prev_record(&self) -> usize {
        self.prev_record as usize
    }

    pub fn num_summaries(&self) -> usize {
        self.num_summaries as usize
    }

    pub fn is_final_record(&self) -> bool {
        self.next_record() == 0
    }
}

#[derive(AsBytes, Clone, Debug, FromBytes)]
#[repr(C)]
pub struct NameRecord {
    raw_names: [u8; RCRD_LEN],
}

impl Default for NameRecord {
    fn default() -> Self {
        Self {
            raw_names: [0_u8; RCRD_LEN],
        }
    }
}

impl NAIFRecord for NameRecord {}

impl NameRecord {
    /// Returns the number of names in this record
    pub fn num_entries(&self, summary_size: usize) -> usize {
        self.raw_names.len() / summary_size * DBL_SIZE
    }

    pub fn nth_name(&self, n: usize, summary_size: usize) -> &str {
        let this_name =
            &self.raw_names[n * summary_size * DBL_SIZE..(n + 1) * summary_size * DBL_SIZE];
        match core::str::from_utf8(this_name) {
            Ok(name) => name.trim(),
            Err(e) => {
                warn!(
                    "malformed name record: `{e}` from {:?}! Using `UNNAMED OBJECT` instead",
                    this_name
                );
                "UNNAMED OBJECT"
            }
        }
    }

    /// Changes the name of the n-th record
    ///
    /// # Safety
    ///
    /// This function uses an `unsafe` call to mutate the underlying `&[u8]` even though it isn't declared as mutable.
    /// This will _only_ change the name record and will, at worst, change the full name.
    ///
    /// In terms of concurrency, this means that if the Context is borrowed while this function is called, between two separate calls,
    /// the name of a record may no longer be available.
    pub fn set_nth_name(&self, n: usize, summary_size: usize, new_name: &str) {
        let this_name =
            &self.raw_names[n * summary_size * DBL_SIZE..(n + 1) * summary_size * DBL_SIZE];

        let this_name = unsafe {
            core::slice::from_raw_parts_mut(this_name.as_ptr() as *mut u8, this_name.len())
        };

        // Copy the name (thanks Clippy)
        let cur_len = this_name.len();
        this_name[..new_name.len().min(cur_len)]
            .copy_from_slice(&new_name.as_bytes()[..new_name.len().min(cur_len)]);

        // Set the rest of the data to spaces.
        for mut_char in this_name.iter_mut().skip(new_name.len()) {
            *mut_char = " ".as_bytes()[0];
        }
    }

    /// Searches the name record for the provided name.
    ///
    /// **Warning:** this performs an O(N) search!
    pub fn index_from_name(&self, name: &str, summary_size: usize) -> Result<usize, AniseError> {
        for i in 0..self.num_entries(summary_size) {
            if self.nth_name(i, summary_size) == name {
                return Ok(i);
            }
        }
        Err(AniseError::ItemNotFound)
    }
}

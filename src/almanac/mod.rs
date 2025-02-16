/*
 * ANISE Toolkit
 * Copyright (C) 2021-2023 Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Documentation: https://nyxspace.com/
 */

use bytes::Bytes;
use log::info;
use snafu::ResultExt;
use std::fs::File;
use zerocopy::FromBytes;

use crate::ephemerides::SPKSnafu;
use crate::errors::{
    AlmanacError, EphemerisSnafu, InputOutputError, LoadingSnafu, OrientationSnafu, TLDataSetSnafu,
};
use crate::file2heap;
use crate::naif::daf::{FileRecord, NAIFRecord};
use crate::naif::{BPC, SPK};
use crate::orientations::BPCSnafu;
use crate::structure::dataset::DataSetType;
use crate::structure::metadata::Metadata;
use crate::structure::{EulerParameterDataSet, PlanetaryDataSet, SpacecraftDataSet};
use core::fmt;

// TODO: Switch these to build constants so that it's configurable when building the library.
pub const MAX_LOADED_SPKS: usize = 32;
pub const MAX_LOADED_BPCS: usize = 8;
pub const MAX_SPACECRAFT_DATA: usize = 16;
pub const MAX_PLANETARY_DATA: usize = 64;

pub mod bpc;
pub mod planetary;
pub mod spk;
pub mod transform;

/// An Almanac contains all of the loaded SPICE and ANISE data.
///
/// # Limitations
/// The stack space required depends on the maximum number of each type that can be loaded.
#[derive(Clone, Default)]
pub struct Almanac {
    /// NAIF SPK is kept unchanged
    pub spk_data: [Option<SPK>; MAX_LOADED_SPKS],
    /// NAIF BPC is kept unchanged
    pub bpc_data: [Option<BPC>; MAX_LOADED_BPCS],
    /// Dataset of planetary data
    pub planetary_data: PlanetaryDataSet,
    /// Dataset of spacecraft data
    pub spacecraft_data: SpacecraftDataSet,
    /// Dataset of euler parameters
    pub euler_param_data: EulerParameterDataSet,
}

impl fmt::Display for Almanac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "Almanac: #SPK = {}\t#BPC = {}",
            self.num_loaded_spk(),
            self.num_loaded_bpc()
        )?;
        if !self.planetary_data.lut.by_id.is_empty() {
            write!(f, "\t{}", self.planetary_data)?;
        }
        if !self.spacecraft_data.lut.by_id.is_empty() {
            write!(f, "\t{}", self.spacecraft_data)?;
        }
        Ok(())
    }
}

impl Almanac {
    /// Loads the provided spacecraft data into a clone of this original Almanac.
    pub fn with_spacecraft_data(&self, spacecraft_data: SpacecraftDataSet) -> Self {
        let mut me = self.clone();
        me.spacecraft_data = spacecraft_data;
        me
    }

    /// Loads the provided Euler parameter data into a clone of this original Almanac.
    pub fn with_euler_parameters(&self, ep_dataset: EulerParameterDataSet) -> Self {
        let mut me = self.clone();
        me.euler_param_data = ep_dataset;
        me
    }

    /// Generic function that tries to load whichever path is provided, guessing to the type.
    pub fn load(&self, path: &str) -> Result<Self, AlmanacError> {
        // Load the data onto the heap
        let bytes = file2heap!(path).with_context(|_| LoadingSnafu {
            path: path.to_string(),
        })?;
        info!("Loading almanac from {path}");
        self.load_from_bytes(bytes)
    }

    pub fn load_from_bytes(&self, bytes: Bytes) -> Result<Self, AlmanacError> {
        // Try to load as a SPICE DAF first (likely the most typical use case)

        // Load the header only
        let file_record = FileRecord::read_from(&bytes[..FileRecord::SIZE]).unwrap();

        if let Ok(fileid) = file_record.identification() {
            match fileid {
                "PCK" => {
                    info!("Loading as DAF/PCK");
                    let bpc = BPC::parse(bytes)
                        .with_context(|_| BPCSnafu {
                            action: "parsing bytes",
                        })
                        .with_context(|_| OrientationSnafu {
                            action: "from generic loading",
                        })?;
                    self.with_bpc(bpc).with_context(|_| OrientationSnafu {
                        action: "adding BPC file to context",
                    })
                }
                "SPK" => {
                    info!("Loading as DAF/SPK");
                    let spk = SPK::parse(bytes)
                        .with_context(|_| SPKSnafu {
                            action: "parsing bytes",
                        })
                        .with_context(|_| EphemerisSnafu {
                            action: "from generic loading",
                        })?;
                    self.with_spk(spk).with_context(|_| EphemerisSnafu {
                        action: "adding SPK file to context",
                    })
                }
                fileid => Err(AlmanacError::GenericError {
                    err: format!("DAF/{fileid} is not yet supported"),
                }),
            }
        } else if let Ok(metadata) = Metadata::decode_header(&bytes) {
            // Now, we can load this depending on the kind of data that it is
            match metadata.dataset_type {
                DataSetType::NotApplicable => unreachable!("no such ANISE data yet"),
                DataSetType::SpacecraftData => {
                    // Decode as spacecraft data
                    let dataset = SpacecraftDataSet::try_from_bytes(bytes).with_context(|_| {
                        TLDataSetSnafu {
                            action: "loading as spacecraft data",
                        }
                    })?;
                    Ok(self.with_spacecraft_data(dataset))
                }
                DataSetType::PlanetaryData => {
                    // Decode as planetary data
                    let dataset = PlanetaryDataSet::try_from_bytes(bytes).with_context(|_| {
                        TLDataSetSnafu {
                            action: "loading as planetary data",
                        }
                    })?;
                    Ok(self.with_planetary_data(dataset))
                }
                DataSetType::EulerParameterData => {
                    // Decode as euler paramater data
                    let dataset =
                        EulerParameterDataSet::try_from_bytes(bytes).with_context(|_| {
                            TLDataSetSnafu {
                                action: "loading Euler parameters",
                            }
                        })?;
                    Ok(self.with_euler_parameters(dataset))
                }
            }
        } else {
            Err(AlmanacError::GenericError {
                err: "Provided file cannot be inspected loaded directly in ANISE and may need a conversion first".to_string(),
            })
        }
    }
}

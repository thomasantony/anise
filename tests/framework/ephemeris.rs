use super::Validator;

/*
 * ANISE Toolkit
 * Copyright (C) 2021-2022 Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Documentation: https://nyxspace.com/
 */

use anise::prelude::*;
use arrow::{
    array::{ArrayRef, Float64Array, StringArray},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use parquet::{arrow::ArrowWriter, file::properties::WriterProperties};
use polars::prelude::*;
use std::{fs::File, io::Read, sync::Arc};

const COMPONENT: &[&'static str] = &["X", "Y", "Z", "VX", "VY", "VZ"];

const BATCH_SIZE: usize = 10_000;

#[derive(Default)]
pub struct EphemValData {
    pub src_frame: String,
    pub dst_frame: String,
    pub epoch_offset: f64,
    pub spice_val_x_km: f64,
    pub anise_val_x_km: f64,
    pub spice_val_y_km: f64,
    pub anise_val_y_km: f64,
    pub spice_val_z_km: f64,
    pub anise_val_z_km: f64,

    pub spice_val_vx_km_s: f64,
    pub anise_val_vx_km_s: f64,
    pub spice_val_vy_km_s: f64,
    pub anise_val_vy_km_s: f64,
    pub spice_val_vz_km_s: f64,
    pub anise_val_vz_km_s: f64,
}

impl EphemValData {
    pub fn error(src_frame: String, dst_frame: String, epoch_offset: f64) -> Self {
        Self {
            src_frame,
            dst_frame,
            epoch_offset,
            spice_val_x_km: f64::INFINITY,
            anise_val_x_km: f64::INFINITY,
            spice_val_y_km: f64::INFINITY,
            anise_val_y_km: f64::INFINITY,
            spice_val_z_km: f64::INFINITY,
            anise_val_z_km: f64::INFINITY,
            spice_val_vx_km_s: f64::INFINITY,
            anise_val_vx_km_s: f64::INFINITY,
            spice_val_vy_km_s: f64::INFINITY,
            anise_val_vy_km_s: f64::INFINITY,
            spice_val_vz_km_s: f64::INFINITY,
            anise_val_vz_km_s: f64::INFINITY,
        }
    }
}

#[derive(Default)]
pub struct EphemerisValidator {
    pub input_file_names: Vec<String>,
    pub output_file_name: String,
    pub writer: Option<ArrowWriter<File>>,
    pub batch_src_frame: Vec<String>,
    pub batch_dst_frame: Vec<String>,
    pub batch_component: Vec<String>,
    pub batch_epoch_offset: Vec<f64>,
    pub batch_spice_val: Vec<f64>,
    pub batch_anise_val: Vec<f64>,
}

impl EphemerisValidator {
    pub fn setup(&mut self) {
        // Build the schema
        let schema = Schema::new(vec![
            Field::new("source frame", DataType::Utf8, false),
            Field::new("destination frame", DataType::Utf8, false),
            Field::new("component", DataType::Utf8, false),
            Field::new("File delta T (s)", DataType::Float64, false),
            Field::new("SPICE value", DataType::Float64, false),
            Field::new("ANISE value", DataType::Float64, false),
        ]);

        let file = File::create(format!("target/{}.parquet", self.output_file_name)).unwrap();

        // Default writer properties
        let props = WriterProperties::builder().build();
        let writer = ArrowWriter::try_new(file, Arc::new(schema), Some(props)).unwrap();

        self.writer = Some(writer);
    }

    /// Executes this ephemeris validation
    pub fn execute<'a, V: Validator<'a, Data = EphemValData>>(mut self) {
        // Load the context here to prevent any memory leak.

        let mut ctx = Context::default();

        let mut buffers: Vec<Vec<u8>> = Vec::with_capacity(self.input_file_names.len());
        let mut spks: Vec<SPK> = Vec::with_capacity(self.input_file_names.len());

        for (i, path) in self.input_file_names.iter().enumerate() {
            // Open the DE file
            let mut file = File::open(path).unwrap();
            file.read_to_end(&mut buffers[i]).unwrap();
        }

        for buf in &buffers {
            spks.push(SPK::parse(buf).unwrap());
        }

        for spk in &spks {
            ctx = ctx.load_spk(spk).unwrap();
        }

        let mut validator: V = V::setup(&self.input_file_names, ctx);

        // Enumeration on the validator shall return the next item.
        for (i, data) in (&mut validator).enumerate() {
            for (j, component) in COMPONENT.iter().enumerate() {
                self.batch_src_frame.push(data.src_frame.clone());
                self.batch_dst_frame.push(data.dst_frame.clone());
                self.batch_component.push(component.to_string());
                self.batch_epoch_offset.push(data.epoch_offset);
                let (spice_val, anise_val) = match j {
                    0 => (data.spice_val_x_km, data.anise_val_x_km),
                    1 => (data.spice_val_y_km, data.anise_val_y_km),
                    2 => (data.spice_val_z_km, data.anise_val_z_km),
                    3 => (data.spice_val_vy_km_s, data.anise_val_vy_km_s),
                    4 => (data.spice_val_vz_km_s, data.anise_val_vz_km_s),
                    5 => (data.spice_val_vz_km_s, data.anise_val_vz_km_s),
                    _ => unreachable!(),
                };
                self.batch_spice_val.push(spice_val);
                self.batch_anise_val.push(anise_val);
            }

            // Consider writing the batch
            if i % BATCH_SIZE == 0 {
                self.persist();
            }
        }
        // Test is finished, so let's close the writer, open it as a lazy dataframe, and pass it to the validation
        self.persist();
        self.writer.unwrap().close().unwrap();
        // Open the parquet file with all the data
        let df = LazyFrame::scan_parquet(
            format!("target/{}.parquet", self.output_file_name),
            Default::default(),
        )
        .unwrap();
        // And perform the validation
        validator.validate(df);
        validator.teardown();
    }

    fn persist(&mut self) {
        self.writer
            .as_mut()
            .unwrap()
            .write(
                &RecordBatch::try_from_iter(vec![
                    (
                        "source frame",
                        Arc::new(StringArray::from(self.batch_src_frame.clone())) as ArrayRef,
                    ),
                    (
                        "destination frame",
                        Arc::new(StringArray::from(self.batch_dst_frame.clone())) as ArrayRef,
                    ),
                    (
                        "component",
                        Arc::new(StringArray::from(self.batch_component.clone())) as ArrayRef,
                    ),
                    (
                        "File delta T (s)",
                        Arc::new(Float64Array::from(self.batch_epoch_offset.clone())) as ArrayRef,
                    ),
                    (
                        "SPICE value",
                        Arc::new(Float64Array::from(self.batch_spice_val.clone())) as ArrayRef,
                    ),
                    (
                        "ANISE value",
                        Arc::new(Float64Array::from(self.batch_anise_val.clone())) as ArrayRef,
                    ),
                ])
                .unwrap(),
            )
            .unwrap();

        // Regularly flush to not lose data
        self.writer.as_mut().unwrap().flush().unwrap();

        // Re-init all of the vectors
        self.batch_src_frame = Vec::with_capacity(BATCH_SIZE);
        self.batch_dst_frame = Vec::with_capacity(BATCH_SIZE);
        self.batch_component = Vec::with_capacity(BATCH_SIZE);
        self.batch_epoch_offset = Vec::with_capacity(BATCH_SIZE);
        self.batch_spice_val = Vec::with_capacity(BATCH_SIZE);
        self.batch_anise_val = Vec::with_capacity(BATCH_SIZE);
    }
}

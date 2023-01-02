/*
 * ANISE Toolkit
 * Copyright (C) 2021-2022 Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Documentation: https://nyxspace.com/
 */

use anise::prelude::Context;
use polars::prelude::LazyFrame;

/// All validation of ANISE computations compared to SPICE must implement the Validator.
///
/// This allows running the validation, outputting all of the data into a Parquet file for post-analysis, and also validating the input.
pub trait Validator<'a>: Iterator<Item = Self::Data> {
    type Data;
    fn setup(files: &[String], ctx: Context<'a>) -> Self;
    /// Process the dataframe and performs all asserts in this function. You may also clone this to store some outlier.
    fn validate(&self, df: LazyFrame);
    // A teardown function that takes ownership of self.
    fn teardown(self);
}

pub mod ephemeris;

#[test]
fn demo() {}

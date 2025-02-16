/*
 * ANISE Toolkit
 * Copyright (C) 2021-2023 Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Documentation: https://nyxspace.com/
 */

use super::{compare::*, validate::Validation};

#[ignore = "Requires Rust SPICE -- must be executed serially"]
#[test]
fn validate_jplde_de440_full() {
    let file_name = "spk-type2-validation-de440".to_string();
    let comparator =
        CompareEphem::new(vec!["data/de440.bsp".to_string()], file_name.clone(), 1_000);

    let err_count = comparator.run();

    assert_eq!(err_count, 0, "None of the queries should fail!");

    let validator = Validation {
        file_name,
        ..Default::default()
    };

    validator.validate();
}

#[ignore = "Requires Rust SPICE -- must be executed serially"]
#[test]
fn validate_jplde_de440s() {
    let output_file_name = "spk-type2-validation-de440s".to_string();
    let comparator = CompareEphem::new(
        vec!["data/de440s.bsp".to_string()],
        output_file_name.clone(),
        1_000,
    );

    let err_count = comparator.run();

    assert_eq!(err_count, 0, "None of the queries should fail!");

    let validator = Validation {
        file_name: output_file_name,
        ..Default::default()
    };

    validator.validate();
}

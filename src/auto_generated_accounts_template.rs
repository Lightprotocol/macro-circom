pub const AUTO_GENERATED_ACCOUNTS_TEMPLATE: &str = "use anchor_lang::prelude::*;

/**
 * This file is auto-generated by the Light cli.
 * DO NOT EDIT MANUALLY.
 * THE FILE WILL BE OVERWRITTEN EVERY TIME THE LIGHT CLI BUILD IS RUN.
 */
    #[allow(non_camel_case_types)]
    // helper struct to create anchor idl with u256 type
    #[account]
    #[derive(Debug, Copy, PartialEq)]
    pub struct u256 {
        pub x: [u8; 32],
    }
 ";

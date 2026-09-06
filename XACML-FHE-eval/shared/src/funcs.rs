use std::fs::OpenOptions;
use csv::Writer;

use tfhe::{ClientKey, ConfigBuilder, PublicKey, ServerKey, generate_keys};
use tfhe::shortint::parameters::{
    COMP_PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M64, COMP_PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128, PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M64, PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128
};

pub fn create_keys() -> (ClientKey, PublicKey, ServerKey) {
	
	let config =
        tfhe::ConfigBuilder::with_custom_parameters(PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128)
            .enable_compression(COMP_PARAM_MESSAGE_2_CARRY_2_KS_PBS_TUNIFORM_2M128)
            .build();
	// let config = ConfigBuilder::default();
	let (ck,sks) = generate_keys(config);
	let pk = PublicKey::new(&ck);
	(ck, pk, sks)
}

pub fn append_to_csv(
    path: String,
    row: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;

    let mut wtr = Writer::from_writer(file);

    wtr.write_record(&row)?;
    wtr.flush()?; // ensure write to disk

    Ok(())
}
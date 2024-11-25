use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

pub fn make_compute_budget_ixs(
    price: u64,
    max_units: u32,
) -> Vec<Instruction> {
    vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(price),
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(max_units),
    ]
}

pub fn create_ata_token_or_not(
    funding: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    token_program: Option<&Pubkey>,
) -> Vec<Instruction> {
    vec![
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            funding,
            owner,
            mint,
            token_program.unwrap_or(&spl_token::id()),
        ),
    ]
}
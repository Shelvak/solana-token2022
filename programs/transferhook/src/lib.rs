use anchor_lang::prelude::*;

declare_id!("7aeu4HRHR4UwQndRDyh5f7nMwgxgH3rrtLgRntxdivZw");

// Sha256(spl-transfer-hook-interface:execute)[..8]
pub const EXECUTE_IX_TAG_LE: [u8; 8] = [105, 37, 101, 197, 75, 251, 102, 26];

#[program]
pub mod transferhook {
    use solana_program::program::invoke_signed;
    use solana_program::system_instruction;
    use spl_pod::primitives::PodBool;
    use spl_tlv_account_resolution::account::ExtraAccountMeta;
    use spl_tlv_account_resolution::state::ExtraAccountMetaList;
    use spl_transfer_hook_interface::collect_extra_account_metas_signer_seeds;
    use spl_transfer_hook_interface::instruction::ExecuteInstruction;

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, paused: bool) -> Result<()> {
        let state = &mut ctx.accounts.state;
        msg!("State programID: {}", state.key());
        state.owner = *ctx.accounts.authority.key;
        state.paused = paused;
        Ok(())
    }

    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        let state = &ctx.accounts.state;

        msg!("State programID: {}", state.key());
        msg!("Transfer hook is paused: {}", state.paused);

        if state.paused {
            msg!("Transfer hook is paused");
            return Err(anchor_lang::prelude::ProgramError::Custom(123).into());
        }

        Ok(())
    }

    pub fn change_paused_state(ctx: Context<ChangePausedState>, paused: bool) -> Result<()> {
        msg!("Changing paused state to {}", paused);

        // Check authority.
        if *ctx.accounts.authority.key != ctx.accounts.state.owner {
            msg!("Authority does not match state owner");
            return Err(anchor_lang::prelude::ProgramError::Custom(123).into());
        }

        let state = &mut ctx.accounts.state;
        state.paused = paused;

        msg!("State programID: {}", state.key());
        msg!("Change paused state to {}", state.paused);

        Ok(())
    }

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
        bump_seed: u8,
    ) -> Result<()> {
        // Create the extra account meta list.
        let account_metas = vec![ExtraAccountMeta {
            discriminator: 0,
            address_config: ctx.accounts.state.key().to_bytes(),
            is_signer: PodBool::from(false),
            is_writable: PodBool::from(true),
        }];

        // Allocate extra account PDA account.
        let bump_seed = [bump_seed];
        let signer_seeds =
            collect_extra_account_metas_signer_seeds(ctx.accounts.mint.key, &bump_seed);
        let account_size = ExtraAccountMetaList::size_of(account_metas.len())?;
        invoke_signed(
            &system_instruction::allocate(ctx.accounts.extra_account.key, account_size as u64),
            &[ctx.accounts.extra_account.clone()],
            &[&signer_seeds],
        )?;
        invoke_signed(
            &system_instruction::assign(ctx.accounts.extra_account.key, ctx.program_id),
            &[ctx.accounts.extra_account.clone()],
            &[&signer_seeds],
        )?;

        // Write the extra account meta list to the extra account PDA.
        let mut data = ctx.accounts.extra_account.try_borrow_mut_data()?;
        ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &account_metas)?;

        msg!(
            "Extra account meta list initialized on {}",
            ctx.accounts.extra_account.key()
        );
        Ok(())
    }

    pub fn fallback(program_id: &Pubkey, accounts: &[AccountInfo], ix_data: &[u8]) -> Result<()> {
        let mut ix_data: &[u8] = ix_data;
        let sighash: [u8; 8] = {
            let mut sighash: [u8; 8] = [0; 8];
            sighash.copy_from_slice(&ix_data[..8]);
            ix_data = &ix_data[8..];
            sighash
        };
        match sighash {
            EXECUTE_IX_TAG_LE => __private::__global::transfer_hook(program_id, accounts, ix_data),
            _ => Err(ProgramError::InvalidInstructionData.into()),
        }
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init,
        seeds = [authority.key().as_ref()],
        bump,
        payer = authority,
        space = 8 + 128)
    ]
    pub state: Account<'info, State>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct TransferHook<'info> {
    /// CHECK:
    pub source: AccountInfo<'info>,
    /// CHECK:
    pub mint: AccountInfo<'info>,
    /// CHECK:
    pub destination: AccountInfo<'info>,
    /// CHECK:
    pub authority: AccountInfo<'info>,
    /// CHECK: must be the extra account PDA
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump)
    ]
    pub extra_account: AccountInfo<'info>,
    /// CHECK:
    pub state: Account<'info, State>,
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    /// CHECK: must be the extra account PDA
    #[account(mut,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump)
    ]
    pub extra_account: AccountInfo<'info>,
    #[account(mut)]
    pub state: Account<'info, State>,
    /// CHECK:
    pub mint: AccountInfo<'info>,
    /// CHECK:
    pub authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct State {
    pub owner: Pubkey,
    pub paused: bool,
}

#[derive(Accounts)]
#[instruction(paused: bool)]
pub struct ChangePausedState<'info> {
    /// CHECK:
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(mut)]
    pub state: Account<'info, State>,
}

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program::invoke,
    system_instruction,
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

// Define the PensionSystem struct to hold contract data
struct PensionSystem {
    owner: Pubkey,
    balances: Vec<(Pubkey, u64)>,
    pension_plan: Vec<(Pubkey, u64)>,
    total_balance: u64,
}

// Define the PensionPlan struct to hold the contributors
#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct PensionPlan {
    owner: Pubkey,
    balances: Vec<(Pubkey, u64)>,
    total_balance: u64,
}

impl PensionSystem {
    fn new(owner: Pubkey) -> Self {
        Self {
            owner,
            balances: Vec::new(),
            pension_plan: Vec::new(),
            total_balance: 0,
        }
    }

    fn contribute(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
    
        if amount == 0 {
            return Err(ProgramError::BorshIoError("Contribution amount must be greater than zero.".to_string()));
        }
    
        let mut pension_plan = PensionPlan::try_from_slice(&sender.data.borrow())?;
        
        if let Some((_, balance)) = pension_plan.balances.iter_mut().find(|(pubkey, _)| pubkey == &sender.key) {
            *balance += amount;
        } else {
            pension_plan.balances.push((sender.key, amount));
        }
    
        let transfer_instruction = system_instruction::transfer(sender.key, &pension_plan.owner, amount);
        invoke(&transfer_instruction, accounts)?;
    
        pension_plan.total_balance += amount;
    
        msg!("New contribution from {:?}: {:?}", sender.key, amount);
    
        pension_plan.serialize(&mut &mut sender.data.borrow_mut()[..])?;
    
        Ok(())
    }
    
    
    fn set_pension_plan(&mut self, sender: &Pubkey, amount: u64) -> ProgramResult {
        if amount == 0 {
            return Err(solana_program::program_error::ProgramError::BorshIoError("Pension plan amount must be greater than zero.".to_string()));
        }

        if let Some((_, balance)) = self.balances.iter_mut().find(|(pubkey, _)| pubkey == sender) {
            if *balance < amount {
                return Err(solana_program::program_error::ProgramError::BorshIoError("Insufficient balance to set pension plan amount.".to_string()));
            }
            *balance -= amount;
        } else {
            return Err(solana_program::program_error::ProgramError::BorshIoError("No balance found for the given sender address.".to_string()));
        }

        if let Some((_, plan)) = self.pension_plan.iter_mut().find(|(pubkey, _)| pubkey == sender) {
            *plan = amount;
        } else {
            self.pension_plan.push((*sender, amount));
        }

        msg!("New pension plan set for {:?}: {:?}", sender, amount);
        Ok(())
    }

    fn get_pension(&mut self, sender: &Pubkey) -> ProgramResult {
        if let Some((_, plan)) = self.pension_plan.iter().find(|(pubkey, _)| pubkey == sender) {
            if *plan == 0 {
                return Err(solana_program::program_error::ProgramError::BorshIoError("No pension plan set for this address.".to_string()));
            }

            if self.total_balance < *plan {
                return Err(solana_program::program_error::ProgramError::BorshIoError("Insufficient funds to pay pension.".to_string()));
            }

            self.balances.iter_mut().find(|(pubkey, _)| pubkey == sender).unwrap().1 += *plan;
            self.total_balance -= *plan;

            msg!("Pension paid to {:?}: {:?}", sender, *plan);
        } else {
            return Err(solana_program::program_error::ProgramError::BorshIoError("No pension plan found for the given sender address.".to_string()));
        }

        Ok(())
    }

    fn get_balance(&self, sender: &Pubkey) -> ProgramResult {
        let balance = self.balances.iter().find(|(pubkey, _)| pubkey == sender).map(|(_, balance)| *balance).unwrap_or(0);
        msg!("Balance of {:?}: {:?}", sender, balance);
        Ok(())
    }

    fn get_total_balance(&self) -> ProgramResult {
        msg!("Total balance: {:?}", self.total_balance);
        Ok(())
    }

    fn get_owner(&self) -> ProgramResult {
        msg!("Owner: {:?}", self.owner);
        Ok(())
    }

    fn set_owner(&mut self, sender: &Pubkey, new_owner: &Pubkey) -> ProgramResult {
        if sender != &self.owner {
            return Err(solana_program::program_error::ProgramError::BorshIoError("Only the owner can set a new owner.".to_string()));
        }

        self.owner = *new_owner;
        msg!("New owner: {:?}", self.owner);
        Ok(())
    }
}


// process_instraction
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let owner = next_account_info(account_info_iter)?;
    let sender = next_account_info(account_info_iter)?;

    let mut pension_system = PensionSystem::new(*owner.key);

    match instruction_data[0] {
        0 => {
            let amount = instruction_data[1..9].iter().fold(0, |acc, x| (acc << 8) + *x as u64);
            pension_system.contribute(sender, amount)
        }
        1 => {
            let amount = instruction_data[1..9].iter().fold(0, |acc, x| (acc << 8) + *x as u64);
            pension_system.set_pension_plan(sender, amount)
        }
        2 => pension_system.get_pension(sender),
        3 => pension_system.get_balance(sender),
        4 => pension_system.get_total_balance(),
        5 => pension_system.get_owner(),
        6 => {
            let new_owner = Pubkey::new(&instruction_data[1..33]);
            pension_system.set_owner(sender, &new_owner)
        }
        _ => Err(solana_program::program_error::ProgramError::BorshIoError("Invalid instruction.".to_string())),
    }
}


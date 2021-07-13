use {
    crate::{
        account::{Account, AccountError},
        event::{Event, EventData, EventError},
        ClientId,
    },
    csv::{ReaderBuilder, Trim},
    log::debug,
    std::{
        collections::HashMap,
        convert::TryFrom,
        io::{self, Read, Write},
    },
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("Event error: {0}")]
    EventError(#[from] EventError),
    #[error("Account error: {0}")]
    AccountError(#[from] AccountError),
}

/// Orchestrates multiple client accounts.
#[derive(Debug, Default)]
pub struct Engine {
    accounts: HashMap<ClientId, Account>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn handle_event(&mut self, event: Event) -> Result<(), AccountError> {
        let account = self
            .accounts
            .entry(event.client)
            .or_insert_with(Account::new);
        match event.data {
            EventData::Deposit {
                transaction_id,
                amount,
            } => account.deposit(transaction_id, amount)?,
            EventData::Withdrawal { amount, .. } => account.withdraw(amount)?,
            EventData::Dispute { transaction_id } => account.dispute(transaction_id)?,
            EventData::Resolve { transaction_id } => account.resolve(transaction_id)?,
            EventData::Chargeback { transaction_id } => account.chargeback(transaction_id)?,
        }
        Ok(())
    }

    pub fn read_events(&mut self, reader: impl Read) -> Result<(), EngineError> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .trim(Trim::All)
            .from_reader(reader);

        for event in reader.records() {
            if let Err(e) = self.handle_event(Event::try_from(event?)?) {
                debug!("Failed to handle event: {}", e);
            }
        }

        Ok(())
    }

    pub fn write_accounts_state(&self, mut writer: impl Write) -> Result<(), io::Error> {
        writeln!(writer, "client,available,held,total,locked")?;

        for (client, account) in &self.accounts {
            writeln!(
                writer,
                "{},{},{},{},{}",
                client,
                account.available_funds(),
                account.held_funds(),
                account.total_funds(),
                account.is_locked(),
            )?;
        }

        Ok(())
    }
}

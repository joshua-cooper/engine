use {
    crate::{Amount, ClientId, TransactionId},
    csv::StringRecord,
    std::{convert::TryFrom, num::ParseIntError},
    thiserror::Error,
};

const DEPOSIT: &str = "deposit";
const WITHDRAWAL: &str = "withdrawal";
const DISPUTE: &str = "dispute";
const RESOLVE: &str = "resolve";
const CHARGEBACK: &str = "chargeback";

#[derive(Debug, Error)]
pub enum EventError {
    #[error("Unknown type: \"{0}\"")]
    UnknownType(String),
    #[error("Missing required field \"type\"")]
    MissingType,
    #[error("Missing required field \"client\"")]
    MissingClientId,
    #[error("Missing required field \"tx\"")]
    MissingTransactionId,
    #[error("Missing required field \"amount\"")]
    MissingAmount,
    #[error("Error parsing client: {0}")]
    InvalidClientId(ParseIntError),
    #[error("Error parsing tx: {0}")]
    InvalidTransactionId(ParseIntError),
    #[error("Error parsing amount: {0}")]
    InvalidAmount(rust_decimal::Error),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventData {
    Deposit {
        transaction_id: TransactionId,
        amount: Amount,
    },
    Withdrawal {
        transaction_id: TransactionId,
        amount: Amount,
    },
    Dispute {
        transaction_id: TransactionId,
    },
    Resolve {
        transaction_id: TransactionId,
    },
    Chargeback {
        transaction_id: TransactionId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    pub client: ClientId,
    pub data: EventData,
}

impl TryFrom<StringRecord> for Event {
    type Error = EventError;

    fn try_from(event: StringRecord) -> Result<Self, Self::Error> {
        let event_type = event.get(0).ok_or(EventError::MissingType)?;
        let client = event
            .get(1)
            .ok_or(EventError::MissingClientId)?
            .parse()
            .map_err(EventError::InvalidClientId)?;
        let transaction_id = event
            .get(2)
            .ok_or(EventError::MissingTransactionId)?
            .parse()
            .map_err(EventError::InvalidTransactionId)?;
        let amount = event
            .get(3)
            .map(|x| x.parse().map_err(EventError::InvalidAmount));

        let data = match (event_type, amount) {
            (DEPOSIT, None) | (WITHDRAWAL, None) => return Err(EventError::MissingAmount),
            (DEPOSIT, Some(amount)) => EventData::Deposit {
                transaction_id,
                amount: amount?,
            },
            (WITHDRAWAL, Some(amount)) => EventData::Withdrawal {
                transaction_id,
                amount: amount?,
            },
            (DISPUTE, _) => EventData::Dispute { transaction_id },
            (RESOLVE, _) => EventData::Resolve { transaction_id },
            (CHARGEBACK, _) => EventData::Chargeback { transaction_id },
            (unknown, _) => return Err(EventError::UnknownType(unknown.to_owned())),
        };

        Ok(Self { client, data })
    }
}

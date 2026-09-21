//! CSV export of ledger entries for spreadsheets. Uses `;` and a decimal
//! comma with a UTF-8 BOM, which is what Excel expects in a pt-BR locale.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use domain::EntryKind;
use domain::money_format::format_decimal_comma;

use super::{AccountService, CardService, CategoryService, LedgerService, MemberService};
use crate::model::{AccountId, CategoryId, EntryFilter, InvoiceId, LedgerEntry, MemberId};
use crate::ports::CardStore;
use crate::{AppError, AppResult};

/// The most rows one export returns; a couple's ledger stays far below.
pub const MAX_EXPORT_ROWS: u32 = 50_000;

const HEADER: [&str; 10] = [
    "data",
    "tipo",
    "descrição",
    "categoria",
    "conta",
    "destino",
    "cartão",
    "parcela",
    "valor",
    "quem",
];

pub struct ExportService {
    sources: ExportSources,
}

pub struct ExportSources {
    pub ledger: Arc<LedgerService>,
    pub accounts: Arc<AccountService>,
    pub categories: Arc<CategoryService>,
    pub cards: Arc<CardService>,
    pub members: Arc<MemberService>,
    pub card_store: Arc<dyn CardStore>,
}

/// Names used to fill the text columns.
struct Names {
    accounts: HashMap<AccountId, String>,
    categories: HashMap<CategoryId, String>,
    members: HashMap<MemberId, String>,
    invoice_cards: HashMap<InvoiceId, String>,
}

impl ExportService {
    pub fn new(sources: ExportSources) -> Self {
        Self { sources }
    }

    /// Entries with `from <= date <= to_inclusive`, oldest first, as CSV.
    ///
    /// ```ignore
    /// let csv = exports.entries_csv(cycle.start, cycle.last_day()).await?;
    /// ```
    pub async fn entries_csv(&self, from: NaiveDate, to_inclusive: NaiveDate) -> AppResult<String> {
        let filter = EntryFilter {
            from: Some(from),
            to_inclusive: Some(to_inclusive),
            limit: MAX_EXPORT_ROWS,
            ..EntryFilter::default()
        };
        let mut entries = self.sources.ledger.list(&filter).await?;
        entries.reverse();
        let names = self.names(&entries).await?;
        let rows: Vec<[String; 10]> = entries.iter().map(|entry| row(entry, &names)).collect();
        write_csv(&rows)
    }

    async fn names(&self, entries: &[LedgerEntry]) -> AppResult<Names> {
        let accounts = self.sources.accounts.list(true).await?;
        let categories = self.sources.categories.list(None).await?;
        let members = self.sources.members.list().await?;
        Ok(Names {
            accounts: accounts.into_iter().map(|account| (account.id, account.name)).collect(),
            categories: categories
                .into_iter()
                .map(|category| (category.id, category.name))
                .collect(),
            members: members.into_iter().map(|member| (member.id, member.display_name)).collect(),
            invoice_cards: self.invoice_cards(entries).await?,
        })
    }

    /// Card name behind each invoice the entries point at.
    async fn invoice_cards(
        &self,
        entries: &[LedgerEntry],
    ) -> AppResult<HashMap<InvoiceId, String>> {
        let cards: HashMap<_, _> =
            self.sources.cards.list().await?.into_iter().map(|card| (card.id, card.name)).collect();
        let mut names = HashMap::new();
        for invoice_id in entries.iter().filter_map(|entry| entry.invoice_id) {
            if let Some(invoice) = self.sources.card_store.find_invoice(invoice_id).await? {
                names.insert(invoice_id, cards.get(&invoice.card_id).cloned().unwrap_or_default());
            }
        }
        Ok(names)
    }
}

fn row(entry: &LedgerEntry, names: &Names) -> [String; 10] {
    [
        entry.accounting_date.format("%d/%m/%Y").to_string(),
        kind_name(entry.kind).to_owned(),
        entry.description.clone(),
        name_of(&names.categories, entry.category_id),
        name_of(&names.accounts, entry.account_id),
        name_of(&names.accounts, entry.counter_account_id),
        name_of(&names.invoice_cards, entry.invoice_id),
        entry.installment_no.map(|number| number.to_string()).unwrap_or_default(),
        format_decimal_comma(entry.amount),
        entry
            .created_by
            .and_then(|id| names.members.get(&id).cloned())
            .unwrap_or_else(|| "automático".into()),
    ]
}

fn name_of<K: Eq + std::hash::Hash>(names: &HashMap<K, String>, id: Option<K>) -> String {
    id.and_then(|id| names.get(&id).cloned()).unwrap_or_default()
}

const fn kind_name(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Income => "Entrada",
        EntryKind::Expense => "Gasto",
        EntryKind::Transfer => "Transferência",
        EntryKind::CardInstallment => "Parcela de cartão",
        EntryKind::CardCredit => "Estorno no cartão",
        EntryKind::InvoicePayment => "Pagamento de fatura",
        EntryKind::Refund => "Reembolso",
        EntryKind::AdjustIn => "Ajuste (+)",
        EntryKind::AdjustOut => "Ajuste (-)",
    }
}

fn write_csv(rows: &[[String; 10]]) -> AppResult<String> {
    let mut writer = csv::WriterBuilder::new().delimiter(b';').from_writer(Vec::new());
    let failure = |error: csv::Error| AppError::Storage(format!("could not write CSV: {error}"));
    writer.write_record(HEADER).map_err(failure)?;
    for row in rows {
        writer.write_record(row).map_err(failure)?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|error| AppError::Storage(format!("could not write CSV: {error}")))?;
    Ok(format!("\u{feff}{}", String::from_utf8_lossy(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_uses_semicolons_bom_and_quotes() {
        let row = ["01/03/2026", "Gasto", "pão; leite", "", "", "", "", "", "10,50", "Ana"]
            .map(String::from);
        let csv = write_csv(&[row]).unwrap();
        assert!(csv.starts_with("\u{feff}data;tipo;descrição;"), "{csv}");
        assert!(csv.contains("01/03/2026;Gasto;\"pão; leite\";;;;;;10,50;Ana"), "{csv}");
    }

    #[test]
    fn every_kind_has_a_name() {
        assert!(EntryKind::ALL.iter().all(|kind| !kind_name(*kind).is_empty()));
    }
}

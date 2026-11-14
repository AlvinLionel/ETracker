use crate::{
    application::imports::{
        AppliedImportCategorySuggestion, CsvImportMapping, ImportCategorySuggestion, ImportError,
        ImportReconciliation, ImportRepository, MAX_SOURCE_FILE_BYTES, MAX_WORKBOOK_EXPANDED_BYTES,
        NewImportBatch, StagedCategoryAllocation, detect_csv_delimiter,
    },
    infrastructure::database::Database,
};
use calamine::{Data, DataType, Reader, open_workbook_auto_from_rs};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use uuid::Uuid;

const MIN_CATEGORY_SUGGESTION_OCCURRENCES: usize = 3;
const MAX_CATEGORY_SUGGESTIONS: usize = 12;

fn parse_import_date(value: &str, format: &str) -> Option<String> {
    if format == "rfc3339" {
        return chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|date| date.to_rfc3339());
    }
    let pattern = match format {
        "iso_date" => "%Y-%m-%d",
        "day_month_year_slash" => "%d/%m/%Y",
        "month_day_year_slash" => "%m/%d/%Y",
        "day_month_year_dash" => "%d-%m-%Y",
        _ => return None,
    };
    chrono::NaiveDate::parse_from_str(value, pattern)
        .ok()
        .map(|date| format!("{}T00:00:00Z", date))
}
fn best_import_category_rule<'a>(
    rules: &'a [(String, String, String)],
    description: &str,
    transaction_type: &str,
) -> Option<&'a str> {
    let description = normalize_category_match_text(description);
    let matching = rules
        .iter()
        .filter(|(_, rule_type, term)| {
            let term = normalize_category_match_text(term);
            rule_type == transaction_type && description.contains(&term)
        })
        .collect::<Vec<_>>();
    let longest = matching
        .iter()
        .map(|(_, _, term)| normalize_category_match_text(term).chars().count())
        .max()?;
    let best = matching
        .into_iter()
        .filter(|(_, _, term)| normalize_category_match_text(term).chars().count() == longest)
        .collect::<Vec<_>>();
    let category_ids = best
        .iter()
        .map(|(category_id, _, _)| category_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    if category_ids.len() == 1 {
        best.first().map(|(category_id, _, _)| category_id.as_str())
    } else {
        None
    }
}

fn normalize_category_match_text(value: &str) -> String {
    let mut normalized = String::new();
    for character in value.chars() {
        if character.is_alphanumeric() {
            normalized.extend(character.to_lowercase());
        } else if !normalized.is_empty() && !normalized.ends_with(' ') {
            normalized.push(' ');
        }
    }
    normalized
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn category_suggestion_phrases(description: &str) -> Vec<String> {
    const GENERIC_TERMS: &[&str] = &[
        "account",
        "bill",
        "buy",
        "charge",
        "charges",
        "credit",
        "credited",
        "customer",
        "deposit",
        "deposits",
        "deposited",
        "debit",
        "debited",
        "fee",
        "fees",
        "goods",
        "m",
        "merchant",
        "mobile",
        "money",
        "mpesa",
        "no",
        "number",
        "id",
        "paid",
        "pay",
        "payment",
        "paybill",
        "pesa",
        "purchase",
        "push",
        "ref",
        "reference",
        "receive",
        "received",
        "send",
        "sent",
        "stk",
        "through",
        "till",
        "transaction",
        "transfer",
        "via",
        "was",
        "were",
        "will",
        "withdraw",
        "withdrawal",
        "withdrawals",
        "withdrawn",
    ];
    const CONNECTOR_TERMS: &[&str] = &[
        "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "in", "is", "it", "of",
        "on", "the", "that", "this", "to", "with", "you", "your",
    ];
    let terms = description
        .split_whitespace()
        .filter(|term| {
            !term.chars().any(char::is_numeric)
                && !GENERIC_TERMS.contains(term)
                && term.chars().count() >= 2
        })
        .take(4)
        .collect::<Vec<_>>();
    let mut phrases = std::collections::BTreeSet::new();
    for phrase_size in (1..=terms.len()).rev() {
        for phrase in terms.windows(phrase_size) {
            if CONNECTOR_TERMS.contains(&phrase[0])
                || CONNECTOR_TERMS.contains(&phrase[phrase.len() - 1])
            {
                continue;
            }
            let phrase = phrase.join(" ");
            if phrase.chars().count() >= 3 && phrase.chars().count() <= 100 {
                phrases.insert(phrase);
            }
        }
    }
    phrases.into_iter().collect()
}

fn category_name_from_match_text(match_text: &str) -> String {
    match_text
        .split_whitespace()
        .map(|term| {
            let mut characters = term.chars();
            characters
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + characters.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone)]
struct UncategorizedImportedTransaction {
    record_id: String,
    transaction_id: String,
    batch_id: String,
    transaction_type: String,
    description: String,
    amount_minor: i64,
}

fn active_import_category_rules(
    connection: &Connection,
) -> Result<Vec<(String, String, String)>, ImportError> {
    let mut statement = connection
        .prepare(
            "SELECT r.category_id,r.category_type,r.match_text FROM import_category_rule_terms r JOIN categories c ON c.id=r.category_id WHERE c.archived_at IS NULL ORDER BY length(r.match_text) DESC,r.created_at,r.id",
        )
        .map_err(|error| ImportError::Database(error.to_string()))?;
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|error| ImportError::Database(error.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| ImportError::Database(error.to_string()))
}

fn unallocated_imported_transactions(
    connection: &Connection,
) -> Result<Vec<UncategorizedImportedTransaction>, ImportError> {
    let mut statement = connection
        .prepare(
            "SELECT r.id,t.id,r.batch_id,r.transaction_type,t.description,t.amount_minor FROM imported_records r JOIN import_batches b ON b.id=r.batch_id AND b.status='finalized' JOIN financial_transactions t ON t.id=r.linked_transaction_id AND t.status='posted' AND t.source_type='import' AND t.transaction_type=r.transaction_type WHERE r.transaction_type IN ('income','expense') AND trim(COALESCE(t.description,''))<>'' AND NOT EXISTS(SELECT 1 FROM category_allocations a WHERE a.transaction_id=t.id) AND NOT EXISTS(SELECT 1 FROM imported_record_allocations a WHERE a.imported_record_id=r.id) ORDER BY r.created_at,r.id",
        )
        .map_err(|error| ImportError::Database(error.to_string()))?;
    statement
        .query_map([], |row| {
            Ok(UncategorizedImportedTransaction {
                record_id: row.get(0)?,
                transaction_id: row.get(1)?,
                batch_id: row.get(2)?,
                transaction_type: row.get(3)?,
                description: row.get(4)?,
                amount_minor: row.get(5)?,
            })
        })
        .map_err(|error| ImportError::Database(error.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| ImportError::Database(error.to_string()))
}

fn build_category_suggestions(
    transactions: &[UncategorizedImportedTransaction],
    rules: &[(String, String, String)],
) -> Vec<ImportCategorySuggestion> {
    let mut counts = std::collections::BTreeMap::<(String, String), usize>::new();
    for transaction in transactions {
        if best_import_category_rule(
            rules,
            &transaction.description,
            &transaction.transaction_type,
        )
        .is_some()
        {
            continue;
        }
        let normalized_description = normalize_category_match_text(&transaction.description);
        for match_text in category_suggestion_phrases(&normalized_description) {
            if normalized_description.contains(&match_text) {
                *counts
                    .entry((transaction.transaction_type.clone(), match_text))
                    .or_default() += 1;
            }
        }
    }
    let mut candidates = counts
        .into_iter()
        .filter(|(_, count)| *count >= MIN_CATEGORY_SUGGESTION_OCCURRENCES)
        .map(|((category_type, match_text), occurrence_count)| {
            (category_type, match_text, occurrence_count)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .1
            .split_whitespace()
            .count()
            .cmp(&left.1.split_whitespace().count())
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.cmp(&right.1))
    });
    let mut suggestions: Vec<ImportCategorySuggestion> = Vec::new();
    for (category_type, match_text, occurrence_count) in candidates {
        let padded_match = format!(" {match_text} ");
        if suggestions.iter().any(|suggestion| {
            suggestion.category_type == category_type
                && format!(" {} ", suggestion.match_text).contains(&padded_match)
        }) {
            continue;
        }
        suggestions.push(ImportCategorySuggestion {
            category_name: category_name_from_match_text(&match_text),
            category_type,
            match_text,
            occurrence_count,
        });
    }
    suggestions.sort_by(|left, right| {
        right
            .occurrence_count
            .cmp(&left.occurrence_count)
            .then_with(|| left.category_type.cmp(&right.category_type))
            .then_with(|| left.match_text.cmp(&right.match_text))
    });
    suggestions.truncate(MAX_CATEGORY_SUGGESTIONS);
    suggestions
}
pub struct SqliteImportRepository<'a> {
    database: &'a Database,
}

fn check_workbook_expanded_size(
    sizes: impl IntoIterator<Item = Result<u64, ImportError>>,
) -> Result<(), ImportError> {
    let mut expanded_bytes = 0_u64;
    for size in sizes {
        let size = size?;
        expanded_bytes = expanded_bytes
            .checked_add(size)
            .ok_or(ImportError::WorkbookTooLarge)?;
        if expanded_bytes > MAX_WORKBOOK_EXPANDED_BYTES {
            return Err(ImportError::WorkbookTooLarge);
        }
    }
    Ok(())
}

fn validate_workbook_expansion(contents: &[u8]) -> Result<(), ImportError> {
    if contents.len() > MAX_SOURCE_FILE_BYTES {
        return Err(ImportError::SourceFileTooLarge);
    }
    if !contents.starts_with(b"PK\x03\x04") {
        return Ok(()); // Legacy .xls files use OLE containers, not ZIP archives.
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(contents))
        .map_err(|error| ImportError::Database(error.to_string()))?;
    let count = archive.len();
    check_workbook_expanded_size((0..count).map(|index| {
        archive
            .by_index(index)
            .map(|entry| entry.size())
            .map_err(|error| ImportError::Database(error.to_string()))
    }))
}

impl<'a> SqliteImportRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn source_file_contents(&self, batch_id: &str) -> Result<Vec<u8>, ImportError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        connection
            .query_row(
                "UPDATE import_source_files SET expires_at=datetime('now', '+90 days') WHERE batch_id=?1 AND expires_at>CURRENT_TIMESTAMP RETURNING contents",
                [batch_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| ImportError::Database(e.to_string()))?
            .ok_or(ImportError::InvalidBatch)
    }

    pub fn compare_revision(
        &self,
        batch_id: &str,
    ) -> Result<Vec<ImportRevisionChangeRecord>, ImportError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = connection
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let previous: Option<String> = tx
            .query_row(
                "SELECT previous.id FROM import_batches current JOIN import_batches previous ON previous.id=current.revision_of_batch_id AND previous.account_id=current.account_id AND previous.status='finalized' WHERE current.id=?1 AND current.status='review'",
                [batch_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let Some(previous_batch_id) = previous else {
            return Err(ImportError::InvalidBatch);
        };
        let original = load_revision_rows(&tx, &previous_batch_id)?;
        let revised = load_revision_rows(&tx, batch_id)?;
        if original.is_empty() || revised.is_empty() {
            return Err(ImportError::InvalidBatch);
        }
        let mut matched_original = vec![false; original.len()];
        let mut matched_revised = vec![false; revised.len()];
        let mut changes = Vec::new();
        for (current_index, current) in revised.iter().enumerate() {
            let candidates: Vec<usize> = original
                .iter()
                .enumerate()
                .filter(|(index, old)| {
                    !matched_original[*index]
                        && !matched_revised[current_index]
                        && old.exact_match(current)
                })
                .map(|(index, _)| index)
                .collect();
            if candidates.len() == 1 {
                let old_index = candidates[0];
                let reverse_candidates = revised
                    .iter()
                    .enumerate()
                    .filter(|(index, row)| {
                        !matched_revised[*index] && original[old_index].exact_match(row)
                    })
                    .count();
                if reverse_candidates == 1 {
                    matched_original[old_index] = true;
                    matched_revised[current_index] = true;
                    changes.push(RevisionChangeDraft::paired(
                        "unchanged",
                        &original[old_index],
                        current,
                    ));
                }
            }
        }
        for (current_index, current) in revised.iter().enumerate() {
            if matched_revised[current_index] {
                continue;
            }
            let candidates: Vec<usize> = original
                .iter()
                .enumerate()
                .filter(|(index, old)| {
                    !matched_original[*index] && old.same_financial_fields(current)
                })
                .map(|(index, _)| index)
                .collect();
            if candidates.len() == 1 {
                let old_index = candidates[0];
                let reverse_candidates = revised
                    .iter()
                    .enumerate()
                    .filter(|(index, row)| {
                        !matched_revised[*index] && original[old_index].same_financial_fields(row)
                    })
                    .count();
                if reverse_candidates == 1 {
                    matched_original[old_index] = true;
                    matched_revised[current_index] = true;
                    changes.push(RevisionChangeDraft::paired(
                        if original[old_index].exact_match(current) {
                            "unchanged"
                        } else {
                            "modified"
                        },
                        &original[old_index],
                        current,
                    ));
                }
            }
            if matched_revised[current_index] {
                continue;
            }
            let candidates: Vec<usize> = original
                .iter()
                .enumerate()
                .filter(|(index, old)| !matched_original[*index] && old.reference_matches(current))
                .map(|(index, _)| index)
                .collect();
            if candidates.len() == 1 {
                let old_index = candidates[0];
                matched_original[old_index] = true;
                matched_revised[current_index] = true;
                changes.push(RevisionChangeDraft::paired(
                    if original[old_index].exact_match(current) {
                        "unchanged"
                    } else {
                        "modified"
                    },
                    &original[old_index],
                    current,
                ));
            }
        }
        for (current_index, current) in revised.iter().enumerate() {
            if matched_revised[current_index] {
                continue;
            }
            let candidates: Vec<usize> = original
                .iter()
                .enumerate()
                .filter(|(index, old)| {
                    !matched_original[*index] && old.description_matches(current)
                })
                .map(|(index, _)| index)
                .collect();
            if candidates.len() == 1 {
                let old_index = candidates[0];
                let reverse_candidates = revised
                    .iter()
                    .enumerate()
                    .filter(|(index, row)| {
                        !matched_revised[*index] && original[old_index].description_matches(row)
                    })
                    .count();
                if reverse_candidates == 1 {
                    matched_original[old_index] = true;
                    matched_revised[current_index] = true;
                    changes.push(RevisionChangeDraft::paired(
                        if original[old_index].exact_match(current) {
                            "unchanged"
                        } else {
                            "modified"
                        },
                        &original[old_index],
                        current,
                    ));
                }
            }
        }
        for (index, row) in revised.iter().enumerate() {
            if !matched_revised[index] {
                let has_ambiguous_original = original.iter().enumerate().any(|(old_index, old)| {
                    !matched_original[old_index]
                        && (old.reference_matches(row)
                            || old.description_matches(row)
                            || old.same_financial_fields(row))
                });
                changes.push(RevisionChangeDraft::single(
                    if has_ambiguous_original {
                        "ambiguous"
                    } else {
                        "added"
                    },
                    None,
                    Some(row),
                ));
            }
        }
        for (index, row) in original.iter().enumerate() {
            if !matched_original[index] {
                let has_ambiguous_revised =
                    revised.iter().enumerate().any(|(new_index, new_row)| {
                        !matched_revised[new_index]
                            && (row.reference_matches(new_row)
                                || row.description_matches(new_row)
                                || row.same_financial_fields(new_row))
                    });
                changes.push(RevisionChangeDraft::single(
                    if has_ambiguous_revised {
                        "ambiguous"
                    } else {
                        "removed"
                    },
                    Some(row),
                    None,
                ));
            }
        }
        tx.execute(
            "DELETE FROM import_revision_changes WHERE batch_id=?1",
            [batch_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        for change in changes {
            tx.execute(
                "INSERT INTO import_revision_changes(id,batch_id,original_record_id,revised_record_id,change_type,resolution) VALUES (?1,?2,?3,?4,?5,?6)",
                params![change.id, batch_id, change.original_record_id, change.revised_record_id, change.change_type, if change.change_type == "unchanged" { Some("keep_original") } else { None }],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
            if change.change_type == "unchanged" {
                if let Some(record_id) = change.revised_record_id {
                    tx.execute(
                        "UPDATE imported_records SET review_status='skipped',duplicate_status='skip' WHERE id=?1",
                        [record_id],
                    )
                    .map_err(|e| ImportError::Database(e.to_string()))?;
                }
            }
        }
        tx.execute(
            "INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'review','INFO','Compared revised statement against its selected previous import')",
            params![Uuid::new_v4().to_string(), batch_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        drop(connection);
        self.list_revision_changes(batch_id)
    }

    pub fn list_revision_changes(
        &self,
        batch_id: &str,
    ) -> Result<Vec<ImportRevisionChangeRecord>, ImportError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT c.id,c.change_type,c.resolution,c.original_record_id,old.normalized_date,old.normalized_amount_minor,old.transaction_type,old.normalized_description,c.revised_record_id,new.normalized_date,new.normalized_amount_minor,new.transaction_type,new.normalized_description,CASE WHEN old.linked_transaction_id IS NOT NULL AND (t.status='voided' OR EXISTS(SELECT 1 FROM transaction_audit_log audit WHERE audit.transaction_id=t.id AND audit.action IN ('updated','voided','reversed'))) THEN 1 ELSE 0 END,old.source_row_reference,new.source_row_reference FROM import_revision_changes c LEFT JOIN imported_records old ON old.id=c.original_record_id LEFT JOIN financial_transactions t ON t.id=old.linked_transaction_id LEFT JOIN imported_records new ON new.id=c.revised_record_id WHERE c.batch_id=?1 ORDER BY CASE c.change_type WHEN 'removed' THEN 0 WHEN 'modified' THEN 1 WHEN 'ambiguous' THEN 2 WHEN 'added' THEN 3 ELSE 4 END,COALESCE(new.source_row_reference,old.source_row_reference)"
        ).map_err(|e| ImportError::Database(e.to_string()))?;
        statement
            .query_map([batch_id], |row| {
                Ok(ImportRevisionChangeRecord {
                    id: row.get(0)?,
                    change_type: row.get(1)?,
                    resolution: row.get(2)?,
                    original_record_id: row.get(3)?,
                    original_date: row.get(4)?,
                    original_amount_minor: row.get(5)?,
                    original_type: row.get(6)?,
                    original_description: row.get(7)?,
                    revised_record_id: row.get(8)?,
                    revised_date: row.get(9)?,
                    revised_amount_minor: row.get(10)?,
                    revised_type: row.get(11)?,
                    revised_description: row.get(12)?,
                    manual_conflict: row.get(13)?,
                    original_source_row_reference: row.get(14)?,
                    revised_source_row_reference: row.get(15)?,
                })
            })
            .map_err(|e| ImportError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ImportError::Database(e.to_string()))
    }

    pub fn set_revision_resolution(
        &self,
        change_id: &str,
        resolution: &str,
    ) -> Result<(), ImportError> {
        if !matches!(resolution, "keep_original" | "use_revised" | "merge") {
            return Err(ImportError::InvalidBatch);
        }
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let change: Option<(String, String, Option<String>, Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT c.batch_id,c.change_type,c.original_record_id,c.revised_record_id,b.revision_of_batch_id FROM import_revision_changes c JOIN import_batches b ON b.id=c.batch_id WHERE c.id=?1 AND b.status='review'",
                [change_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let Some((batch_id, change_type, original_id, revised_id, previous_batch_id)) = change
        else {
            return Err(ImportError::InvalidBatch);
        };
        if change_type == "unchanged" {
            return Err(ImportError::InvalidBatch);
        }
        if resolution == "merge" {
            if change_type != "modified" {
                return Err(ImportError::InvalidBatch);
            }
            let previous_batch_id = previous_batch_id.ok_or(ImportError::InvalidBatch)?;
            let old_rows = load_revision_rows(&tx, &previous_batch_id)?;
            let new_rows = load_revision_rows(&tx, &batch_id)?;
            let original = old_rows
                .iter()
                .find(|row| Some(row.id.as_str()) == original_id.as_deref())
                .ok_or(ImportError::InvalidBatch)?;
            let revised = new_rows
                .iter()
                .find(|row| Some(row.id.as_str()) == revised_id.as_deref())
                .ok_or(ImportError::InvalidBatch)?;
            let descriptions_compatible = normalize_description(original.description.as_deref())
                == normalize_description(revised.description.as_deref())
                || original.description.as_deref().is_none_or(str::is_empty)
                || revised.description.as_deref().is_none_or(str::is_empty);
            if original.date != revised.date
                || original.amount_minor != revised.amount_minor
                || original.transaction_type != revised.transaction_type
                || original.allocations != revised.allocations
                || !descriptions_compatible
            {
                return Err(ImportError::InvalidBatch);
            }
            let original_is_posted: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM imported_records r JOIN financial_transactions t ON t.id=r.linked_transaction_id WHERE r.id=?1 AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id))",
                    [original.id.as_str()],
                    |row| row.get(0),
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            let has_linked_transaction: bool = tx
                .query_row(
                    "SELECT linked_transaction_id IS NOT NULL FROM imported_records WHERE id=?1",
                    [original.id.as_str()],
                    |row| row.get(0),
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            if has_linked_transaction && !original_is_posted {
                return Err(ImportError::InvalidBatch);
            }
        }
        if let Some(revised_id) = &revised_id {
            let (validation_status, duplicate_status): (String, String) = tx
                .query_row(
                    "SELECT validation_status,duplicate_status FROM imported_records WHERE id=?1 AND batch_id=?2",
                    params![revised_id, batch_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            if resolution == "use_revised" && validation_status != "valid" {
                return Err(ImportError::InvalidBatch);
            }
            let (review_status, new_duplicate_status) = if resolution == "use_revised" {
                ("confirmed", duplicate_status.as_str())
            } else {
                ("skipped", "skip")
            };
            tx.execute(
                "UPDATE imported_records SET review_status=?1,duplicate_status=?2 WHERE id=?3",
                params![review_status, new_duplicate_status, revised_id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        tx.execute(
            "UPDATE import_revision_changes SET resolution=?1 WHERE id=?2",
            params![resolution, change_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        tx.execute(
            "INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'review','INFO',?3)",
            params![Uuid::new_v4().to_string(), batch_id, format!("Revised statement change resolved as {resolution}")],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))
    }
}
impl ImportRepository for SqliteImportRepository<'_> {
    fn list_category_suggestions(&self) -> Result<Vec<ImportCategorySuggestion>, ImportError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let rules = active_import_category_rules(&connection)?;
        let transactions = unallocated_imported_transactions(&connection)?;
        Ok(build_category_suggestions(&transactions, &rules))
    }

    fn apply_category_suggestion(
        &self,
        category_type: &str,
        match_text: &str,
    ) -> Result<AppliedImportCategorySuggestion, ImportError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = connection
            .transaction()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        let existing_rules = active_import_category_rules(&tx)?;
        let transactions = unallocated_imported_transactions(&tx)?;
        let suggestion = build_category_suggestions(&transactions, &existing_rules)
            .into_iter()
            .find(|suggestion| {
                suggestion.category_type == category_type
                    && suggestion
                        .match_text
                        .eq_ignore_ascii_case(match_text.trim())
            })
            .ok_or(ImportError::InvalidBatch)?;

        let category_id: Option<String> = tx
            .query_row(
                "SELECT id FROM categories WHERE lower(name)=lower(?1) AND category_type=?2 AND archived_at IS NULL",
                params![suggestion.category_name, category_type],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        let category_id = if let Some(category_id) = category_id {
            category_id
        } else {
            let category_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO categories(id,name,category_type,is_system) VALUES (?1,?2,?3,0)",
                params![category_id, suggestion.category_name, category_type],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
            category_id
        };
        let existing_term: Option<String> = tx
            .query_row(
                "SELECT category_id FROM import_category_rule_terms WHERE category_type=?1 AND lower(match_text)=lower(?2)",
                params![category_type, suggestion.match_text],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        match existing_term {
            Some(existing_category_id) if existing_category_id != category_id => {
                return Err(ImportError::InvalidBatch);
            }
            Some(_) => {}
            None => {
                tx.execute(
                    "INSERT INTO import_category_rule_terms(id,category_id,category_type,match_text) VALUES (?1,?2,?3,?4)",
                    params![Uuid::new_v4().to_string(), category_id, category_type, suggestion.match_text],
                )
                .map_err(|error| ImportError::Database(error.to_string()))?;
            }
        }

        let rules = active_import_category_rules(&tx)?;
        let mut batches = std::collections::HashSet::new();
        let mut categorized_transaction_count = 0;
        for imported in &transactions {
            if best_import_category_rule(&rules, &imported.description, &imported.transaction_type)
                != Some(category_id.as_str())
            {
                continue;
            }
            tx.execute(
                "INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)",
                params![Uuid::new_v4().to_string(), imported.transaction_id, category_id, imported.amount_minor],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
            tx.execute(
                "INSERT INTO imported_record_allocations(id,imported_record_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)",
                params![Uuid::new_v4().to_string(), imported.record_id, category_id, imported.amount_minor],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
            tx.execute(
                "UPDATE financial_transactions SET updated_at=CURRENT_TIMESTAMP WHERE id=?1",
                [&imported.transaction_id],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
            tx.execute(
                "INSERT INTO transaction_audit_log(id,transaction_id,action,previous_values,new_values,reason) VALUES (?1,?2,'updated',?3,?4,'Accepted a repeated-description import category suggestion')",
                params![
                    Uuid::new_v4().to_string(),
                    imported.transaction_id,
                    serde_json::json!({"allocations": []}).to_string(),
                    serde_json::json!({"allocations": [{"category_id": category_id, "amount_minor": imported.amount_minor}]}).to_string()
                ],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
            batches.insert(imported.batch_id.as_str());
            categorized_transaction_count += 1;
        }
        if categorized_transaction_count == 0 {
            return Err(ImportError::InvalidBatch);
        }
        for batch_id in batches {
            tx.execute(
                "INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'categorization','INFO',?3)",
                params![
                    Uuid::new_v4().to_string(),
                    batch_id,
                    format!(
                        "Applied the {} category suggestion to imported transactions",
                        suggestion.category_name
                    )
                ],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        }
        tx.commit()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        Ok(AppliedImportCategorySuggestion {
            category_name: suggestion.category_name,
            category_type: suggestion.category_type,
            match_text: suggestion.match_text,
            categorized_transaction_count,
        })
    }

    fn create_batch(&self, batch: &NewImportBatch) -> Result<String, ImportError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let active: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND archived_at IS NULL)",
                [&batch.account_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if !active {
            return Err(ImportError::InvalidBatch);
        }
        if let Some(previous_batch_id) = &batch.revision_of_batch_id {
            let eligible: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=?1 AND account_id=?2 AND status='finalized')",
                    params![previous_batch_id, batch.account_id],
                    |r| r.get(0),
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            if !eligible {
                return Err(ImportError::InvalidBatch);
            }
        }
        let id = Uuid::new_v4().to_string();
        let tx = connection
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let calculated_hash = format!("{:x}", Sha256::digest(&batch.source_file_contents));
        tx.execute("INSERT INTO import_batches(id,account_id,source_file_name,source_file_hash,source_format,revision_of_batch_id) VALUES (?1,?2,?3,?4,?5,?6)",params![id,batch.account_id,batch.source_file_name,calculated_hash,batch.source_format,batch.revision_of_batch_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.execute(
            "INSERT INTO import_source_files(batch_id,contents,sha256) VALUES (?1,?2,?3)",
            params![id, batch.source_file_contents, calculated_hash],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'staging','INFO','Import batch created; source file retained locally for up to 90 days after the latest activity')",params![Uuid::new_v4().to_string(),id]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(id)
    }
    fn correct_record(
        &self,
        record_id: &str,
        date: &str,
        amount_minor: i64,
        kind: &str,
        description: Option<&str>,
    ) -> Result<(), ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let batch:Option<String>=tx.query_row("SELECT b.id FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=?1 AND b.status='review'",[record_id],|r|r.get(0)).optional().map_err(|e|ImportError::Database(e.to_string()))?;
        let Some(batch) = batch else {
            return Err(ImportError::InvalidBatch);
        };
        tx.execute("UPDATE imported_records SET normalized_date=?1,normalized_amount_minor=?2,transaction_type=?3,normalized_description=?4,validation_status='valid',review_status='corrected' WHERE id=?5",params![date,amount_minor,kind,description,record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        let duplicate: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM imported_records r JOIN import_batches b ON b.id=r.batch_id JOIN accounts a ON a.id=b.account_id JOIN financial_transactions t ON t.transaction_type=r.transaction_type JOIN account_entries e ON e.transaction_id=t.id AND e.account_id=a.id WHERE r.id=?1 AND t.status='posted' AND t.currency_code=a.currency_code AND t.amount_minor=?2 AND date(t.occurred_at)=date(?3) AND lower(trim(COALESCE(t.description,'')))=lower(trim(COALESCE(r.normalized_description,'')))) OR EXISTS(SELECT 1 FROM imported_records r JOIN imported_records other ON other.batch_id=r.batch_id AND other.id<>r.id WHERE r.id=?1 AND other.validation_status='valid' AND other.normalized_amount_minor=?2 AND other.transaction_type=?4 AND date(other.normalized_date)=date(?3) AND lower(trim(COALESCE(other.normalized_description,'')))=lower(trim(COALESCE(r.normalized_description,''))))",
            params![record_id, amount_minor, date, kind], |r| r.get(0),
        ).map_err(|e| ImportError::Database(e.to_string()))?;
        if duplicate {
            tx.execute("UPDATE imported_records SET duplicate_status='probable',review_status='pending' WHERE id=?1", [record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        } else {
            tx.execute("UPDATE imported_records SET duplicate_status='unmatched',review_status='corrected' WHERE id=?1", [record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        }
        let message = if duplicate {
            "Staged record corrected; possible duplicate requires review"
        } else {
            "Staged record corrected"
        };
        let previous_reconciliation: Option<(Option<i64>, Option<i64>)> = tx
            .query_row(
                "SELECT opening_balance_minor,closing_balance_minor FROM import_reconciliations WHERE batch_id=?1",
                [&batch],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if let Some((Some(opening), Some(closing))) = previous_reconciliation {
            match calculate_reconciliation(&tx, &batch, Some(opening), Some(closing)) {
                Ok(result) => {
                    tx.execute("UPDATE import_reconciliations SET expected_closing_balance_minor=?1,difference_minor=?2,outcome=?3,explanation=?4,updated_at=CURRENT_TIMESTAMP WHERE batch_id=?5", params![result.expected_closing_balance_minor,result.difference_minor,result.outcome,result.explanation,batch]).map_err(|e|ImportError::Database(e.to_string()))?;
                    tx.execute(
                        "UPDATE import_batches SET reconciliation_status=?1 WHERE id=?2",
                        params![result.outcome, batch],
                    )
                    .map_err(|e| ImportError::Database(e.to_string()))?;
                }
                Err(ImportError::InvalidBatch) => {
                    tx.execute("UPDATE import_reconciliations SET expected_closing_balance_minor=NULL,difference_minor=NULL,outcome='unavailable',explanation='Reconciliation awaits valid values for every staged row.',updated_at=CURRENT_TIMESTAMP WHERE batch_id=?1", [&batch]).map_err(|e|ImportError::Database(e.to_string()))?;
                    tx.execute(
                        "UPDATE import_batches SET reconciliation_status='unavailable' WHERE id=?1",
                        [&batch],
                    )
                    .map_err(|e| ImportError::Database(e.to_string()))?;
                }
                Err(error) => return Err(error),
            }
        }
        tx.execute("INSERT INTO import_logs(id,batch_id,record_id,stage,severity,message) VALUES (?1,?2,?3,'review',?4,?5)",params![Uuid::new_v4().to_string(),batch,record_id,if duplicate {"WARNING"} else {"INFO"},message]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn review_record(&self, record_id: &str, keep: bool) -> Result<(), ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let batch:Option<String>=tx.query_row("SELECT b.id FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=?1 AND b.status='review' AND r.validation_status='valid'",[record_id],|r|r.get(0)).optional().map_err(|e|ImportError::Database(e.to_string()))?;
        let Some(batch) = batch else {
            return Err(ImportError::InvalidBatch);
        };
        let (review, duplicate) = if keep {
            ("confirmed", "confirmed_new")
        } else {
            ("skipped", "skip")
        };
        tx.execute(
            "UPDATE imported_records SET review_status=?1,duplicate_status=?2 WHERE id=?3",
            params![review, duplicate, record_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        let message = if keep {
            "Candidate confirmed as new"
        } else {
            "Candidate skipped by user"
        };
        tx.execute("INSERT INTO import_logs(id,batch_id,record_id,stage,severity,message) VALUES (?1,?2,?3,'review','INFO',?4)",params![Uuid::new_v4().to_string(),batch,record_id,message]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn confirm_valid_records(&self, batch_id: &str) -> Result<usize, ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let reviewable: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=?1 AND status='review' AND revision_of_batch_id IS NULL)",
                [batch_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if !reviewable {
            return Err(ImportError::InvalidBatch);
        }
        let changed = tx
            .execute(
                "UPDATE imported_records SET review_status='confirmed',duplicate_status='confirmed_new' WHERE batch_id=?1 AND validation_status='valid' AND review_status='pending' AND duplicate_status='unmatched'",
                [batch_id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if changed > 0 {
            tx.execute(
                "INSERT INTO import_logs(id,batch_id,stage,severity,message,corrective_action) VALUES (?1,?2,'review','INFO',?3,'Ambiguous and possible-duplicate rows were left for individual review.')",
                params![Uuid::new_v4().to_string(),batch_id,format!("Bulk confirmed {changed} valid, non-duplicate records")],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(changed)
    }

    fn finalize(&self, batch_id: &str) -> Result<usize, ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let batch: Option<(String, String, Option<String>)> = tx
            .query_row(
                "SELECT account_id,status,revision_of_batch_id FROM import_batches WHERE id=?1",
                [batch_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let Some((account_id, status, revision_of_batch_id)) = batch else {
            return Err(ImportError::InvalidBatch);
        };
        if status != "review" {
            return Err(ImportError::InvalidBatch);
        }
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND archived_at IS NULL)",
                [&account_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if !active {
            return Err(ImportError::InvalidBatch);
        }
        if revision_of_batch_id.is_none() {
            let imported = tx
                .execute(
                    "UPDATE imported_records SET review_status='confirmed',duplicate_status='confirmed_new' WHERE batch_id=?1 AND review_status='pending' AND validation_status='valid' AND transaction_type IN ('income','expense') AND duplicate_status='unmatched'",
                    [batch_id],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            let excluded = tx
                .execute(
                    "UPDATE imported_records SET review_status='skipped',duplicate_status='skip' WHERE batch_id=?1 AND review_status<>'skipped' AND (validation_status<>'valid' OR transaction_type NOT IN ('income','expense') OR duplicate_status NOT IN ('unmatched','confirmed_new'))",
                    [batch_id],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            if imported > 0 || excluded > 0 {
                tx.execute(
                    "INSERT INTO import_logs(id,batch_id,stage,severity,message,corrective_action) VALUES (?1,?2,'review',?3,?4,?5)",
                    params![
                        Uuid::new_v4().to_string(),
                        batch_id,
                        if excluded > 0 { "WARNING" } else { "INFO" },
                        format!("Prepared {imported} eligible transactions; excluded {excluded} invalid, uncertain, or possible-duplicate rows"),
                        "Only valid, direction-classified, unmatched rows are imported automatically. Excluded rows are kept in the workspace history and are not posted.",
                    ],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            }
        }
        if revision_of_batch_id.is_some() {
            let unresolved_changes: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM import_revision_changes WHERE batch_id=?1 AND change_type<>'unchanged' AND resolution IS NULL",
                    [batch_id],
                    |row| row.get(0),
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            if unresolved_changes > 0 {
                return Err(ImportError::InvalidBatch);
            }
            let unresolved_resolution_records: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM import_revision_changes c JOIN imported_records r ON r.id=c.revised_record_id WHERE c.batch_id=?1 AND c.change_type<>'unchanged' AND c.resolution='use_revised' AND r.validation_status<>'valid'",
                    [batch_id],
                    |row| row.get(0),
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            if unresolved_resolution_records > 0 {
                return Err(ImportError::InvalidBatch);
            }
        }
        let unresolved: i64 = if revision_of_batch_id.is_some() {
            tx.query_row("SELECT COUNT(*) FROM imported_records WHERE batch_id=?1 AND (validation_status<>'valid' OR review_status='pending' OR (duplicate_status='probable' AND review_status NOT IN ('confirmed','skipped','corrected')))", [batch_id], |row| row.get(0))
                .map_err(|e| ImportError::Database(e.to_string()))?
        } else {
            tx.query_row("SELECT COUNT(*) FROM imported_records WHERE batch_id=?1 AND review_status<>'skipped' AND (validation_status<>'valid' OR review_status='pending' OR transaction_type NOT IN ('income','expense') OR duplicate_status NOT IN ('unmatched','confirmed_new'))", [batch_id], |row| row.get(0))
                .map_err(|e| ImportError::Database(e.to_string()))?
        };
        if unresolved > 0 {
            return Err(ImportError::InvalidBatch);
        }
        let staged_count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM imported_records WHERE batch_id=?1",
                [batch_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if staged_count == 0 {
            return Err(ImportError::InvalidBatch);
        }
        if revision_of_batch_id.is_some() {
            let mut originals = tx
                .prepare(
                    "SELECT DISTINCT t.id,t.status,t.source_type,EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id) FROM import_revision_changes c JOIN imported_records old ON old.id=c.original_record_id JOIN financial_transactions t ON t.id=old.linked_transaction_id WHERE c.batch_id=?1 AND c.resolution='use_revised'",
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            let rows = originals
                .query_map([batch_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, bool>(3)?,
                    ))
                })
                .map_err(|e| ImportError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| ImportError::Database(e.to_string()))?;
            drop(originals);
            for (original_id, original_status, source_type, already_reversed) in rows {
                if source_type != "import"
                    || !matches!(original_status.as_str(), "posted" | "voided")
                {
                    return Err(ImportError::InvalidBatch);
                }
                if original_status == "voided" || already_reversed {
                    continue;
                }
                let reversal_id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,source_type,reversal_of_transaction_id) SELECT ?1,'reversal',amount_minor,currency_code,datetime('now'),description,'reversal',id FROM financial_transactions WHERE id=?2",
                    params![reversal_id, original_id],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
                tx.execute(
                    "INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) SELECT ?1,?2,account_id,-amount_minor FROM account_entries WHERE transaction_id=?3",
                    params![Uuid::new_v4().to_string(), reversal_id, original_id],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
                tx.execute(
                    "INSERT INTO transaction_audit_log(id,transaction_id,action,new_values,reason) VALUES (?1,?2,'reversed',?3,?4)",
                    params![Uuid::new_v4().to_string(), original_id, serde_json::json!({"reversal_id":reversal_id,"revision_batch_id":batch_id}).to_string(), format!("Replaced through revised statement import {batch_id}")],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            }
            let mut merges = tx
                .prepare(
                    "SELECT t.id,t.description,new.normalized_description FROM import_revision_changes c JOIN imported_records old ON old.id=c.original_record_id JOIN financial_transactions t ON t.id=old.linked_transaction_id JOIN imported_records new ON new.id=c.revised_record_id WHERE c.batch_id=?1 AND c.resolution='merge' AND t.description IS NULL AND new.normalized_description IS NOT NULL",
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            let merge_rows = merges
                .query_map([batch_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| ImportError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| ImportError::Database(e.to_string()))?;
            drop(merges);
            for (transaction_id, previous_description, revised_description) in merge_rows {
                tx.execute(
                    "UPDATE financial_transactions SET description=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                    params![revised_description, transaction_id],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
                tx.execute(
                    "INSERT INTO transaction_audit_log(id,transaction_id,action,previous_values,new_values,reason) VALUES (?1,?2,'updated',?3,?4,?5)",
                    params![Uuid::new_v4().to_string(), transaction_id, serde_json::json!({"description":previous_description}).to_string(), serde_json::json!({"description":revised_description}).to_string(), format!("Merged compatible details from revised statement import {batch_id}")],
                )
                .map_err(|e| ImportError::Database(e.to_string()))?;
            }
        }
        let rows: Vec<(String, i64, String, String, Option<String>)> = {
            let mut s=tx.prepare("SELECT id,normalized_amount_minor,transaction_type,normalized_date,normalized_description FROM imported_records WHERE batch_id=?1 AND review_status<>'skipped' ORDER BY CAST(source_row_reference AS INTEGER)").map_err(|e|ImportError::Database(e.to_string()))?;
            s.query_map([batch_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(|e| ImportError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ImportError::Database(e.to_string()))?
        };
        if rows.is_empty() && revision_of_batch_id.is_none() {
            return Err(ImportError::InvalidBatch);
        }
        let rules = active_import_category_rules(&tx)?;
        let currency: String = tx
            .query_row(
                "SELECT currency_code FROM accounts WHERE id=?1",
                [&account_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        for (record_id, amount, kind, date, description) in &rows {
            if *amount <= 0
                || !matches!(kind.as_str(), "income" | "expense")
                || chrono::DateTime::parse_from_rfc3339(date).is_err()
            {
                return Err(ImportError::InvalidBatch);
            }
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,source_type) VALUES (?1,?2,?3,?4,?5,?6,'import')",params![id,kind,amount,currency,date,description]).map_err(|e|ImportError::Database(e.to_string()))?;
            let signed = if kind == "income" { *amount } else { -*amount };
            tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,account_id,signed]).map_err(|e|ImportError::Database(e.to_string()))?;
            tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'created',?3)",params![Uuid::new_v4().to_string(),id,serde_json::json!({"source":"import","batch_id":batch_id,"record_id":record_id,"amount_minor":amount,"transaction_type":kind,"occurred_at":date}).to_string()]).map_err(|e|ImportError::Database(e.to_string()))?;
            if let Some(description) = description {
                if let Some(category_id) = best_import_category_rule(&rules, description, kind) {
                    tx.execute(
                        "DELETE FROM imported_record_allocations WHERE imported_record_id=?1",
                        [record_id],
                    )
                    .map_err(|e| ImportError::Database(e.to_string()))?;
                    tx.execute(
                        "INSERT INTO imported_record_allocations(id,imported_record_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)",
                        params![Uuid::new_v4().to_string(),record_id,category_id,amount],
                    )
                    .map_err(|e| ImportError::Database(e.to_string()))?;
                }
            }
            let allocations: Vec<(String, i64)> = {
                let mut statement = tx.prepare("SELECT a.category_id,a.amount_minor FROM imported_record_allocations a WHERE a.imported_record_id=?1 ORDER BY a.category_id").map_err(|e|ImportError::Database(e.to_string()))?;
                statement
                    .query_map([record_id], |r| Ok((r.get(0)?, r.get(1)?)))
                    .map_err(|e| ImportError::Database(e.to_string()))?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|e| ImportError::Database(e.to_string()))?
            };
            let allocation_total = allocations
                .iter()
                .try_fold(0i64, |total, (_, value)| total.checked_add(*value))
                .ok_or(ImportError::InvalidBatch)?;
            if !allocations.is_empty() && allocation_total != *amount {
                return Err(ImportError::InvalidBatch);
            }
            for (category_id, allocation_amount) in allocations {
                let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND (category_type=?2 OR (?2='expense' AND category_type='emergency')) AND archived_at IS NULL)", params![category_id,kind], |r|r.get(0)).map_err(|e|ImportError::Database(e.to_string()))?;
                if !valid {
                    return Err(ImportError::InvalidBatch);
                }
                tx.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)", params![Uuid::new_v4().to_string(),id,category_id,allocation_amount]).map_err(|e|ImportError::Database(e.to_string()))?;
            }
            tx.execute(
                "UPDATE imported_records SET linked_transaction_id=?1 WHERE id=?2",
                params![id, record_id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        tx.execute("UPDATE import_batches SET status='finalized',finalized_at=CURRENT_TIMESTAMP WHERE id=?1 AND status='review'",[batch_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'finalize','INFO',?3)",params![Uuid::new_v4().to_string(),batch_id,format!("Finalized {} staged transactions",rows.len())]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(rows.len())
    }

    fn record_failure(
        &self,
        batch_id: &str,
        record_id: Option<&str>,
        stage: &str,
        message: &str,
        corrective_action: &str,
    ) -> Result<(), ImportError> {
        if !matches!(stage, "parse" | "review" | "finalize") {
            return Err(ImportError::InvalidBatch);
        }
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let valid: bool = c
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=?1 AND status='review')",
                [batch_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if !valid {
            return Err(ImportError::InvalidBatch);
        }
        c.execute("INSERT INTO import_logs(id,batch_id,record_id,stage,severity,message,corrective_action) VALUES (?1,?2,?3,?4,'ERROR',?5,?6)", params![Uuid::new_v4().to_string(), batch_id, record_id, stage, message, corrective_action]).map_err(|e|ImportError::Database(e.to_string()))?;
        Ok(())
    }

    fn reconcile(
        &self,
        batch_id: &str,
        opening: Option<i64>,
        closing: Option<i64>,
    ) -> Result<ImportReconciliation, ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=?1 AND status='review')",
                [batch_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if !exists {
            return Err(ImportError::InvalidBatch);
        }
        let result = calculate_reconciliation(&tx, batch_id, opening, closing)?;
        tx.execute("INSERT INTO import_reconciliations(id,batch_id,opening_balance_minor,closing_balance_minor,expected_closing_balance_minor,difference_minor,outcome,explanation) VALUES (?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(batch_id) DO UPDATE SET opening_balance_minor=excluded.opening_balance_minor,closing_balance_minor=excluded.closing_balance_minor,expected_closing_balance_minor=excluded.expected_closing_balance_minor,difference_minor=excluded.difference_minor,outcome=excluded.outcome,explanation=excluded.explanation,updated_at=CURRENT_TIMESTAMP", params![Uuid::new_v4().to_string(),batch_id,result.opening_balance_minor,result.closing_balance_minor,result.expected_closing_balance_minor,result.difference_minor,result.outcome,result.explanation]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.execute(
            "UPDATE import_batches SET reconciliation_status=?1 WHERE id=?2",
            params![result.outcome, batch_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'reconciliation',?3,?4)", params![Uuid::new_v4().to_string(),batch_id,if result.outcome=="discrepancy" {"WARNING"} else {"INFO"},format!("Reconciliation outcome: {}",result.outcome)]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(result)
    }

    fn set_record_allocations(
        &self,
        record_id: &str,
        allocations: &[StagedCategoryAllocation],
    ) -> Result<(), ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let record: Option<(String, i64, String)> = tx.query_row("SELECT r.batch_id,r.normalized_amount_minor,r.transaction_type FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=?1 AND r.validation_status='valid' AND b.status='review'", [record_id], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|ImportError::Database(e.to_string()))?;
        let Some((batch_id, amount, kind)) = record else {
            return Err(ImportError::InvalidBatch);
        };
        let sum = allocations
            .iter()
            .try_fold(0i64, |sum, allocation| {
                sum.checked_add(allocation.amount_minor)
            })
            .ok_or(ImportError::InvalidBatch)?;
        if !allocations.is_empty() && sum != amount {
            return Err(ImportError::InvalidBatch);
        }
        for allocation in allocations {
            let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND (category_type=?2 OR (?2='expense' AND category_type='emergency')) AND archived_at IS NULL)", params![allocation.category_id,kind], |r|r.get(0)).map_err(|e|ImportError::Database(e.to_string()))?;
            if !active {
                return Err(ImportError::InvalidBatch);
            }
        }
        tx.execute(
            "DELETE FROM imported_record_allocations WHERE imported_record_id=?1",
            [record_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        for allocation in allocations {
            tx.execute("INSERT INTO imported_record_allocations(id,imported_record_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)", params![Uuid::new_v4().to_string(),record_id,allocation.category_id,allocation.amount_minor]).map_err(|e|ImportError::Database(e.to_string()))?;
        }
        tx.execute("INSERT INTO import_logs(id,batch_id,record_id,stage,severity,message) VALUES (?1,?2,?3,'review','INFO','Import category allocations updated')", params![Uuid::new_v4().to_string(),batch_id,record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
}
#[derive(serde::Serialize)]
pub struct ImportBatchRecord {
    pub id: String,
    pub account_id: String,
    pub source_file_name: String,
    pub source_format: String,
    pub status: String,
    pub reconciliation_status: String,
    pub created_at: String,
    pub revision_of_batch_id: Option<String>,
}
#[derive(serde::Serialize)]
pub struct ImportRevisionChangeRecord {
    pub id: String,
    pub change_type: String,
    pub resolution: Option<String>,
    pub original_record_id: Option<String>,
    pub original_date: Option<String>,
    pub original_amount_minor: Option<i64>,
    pub original_type: Option<String>,
    pub original_description: Option<String>,
    pub revised_record_id: Option<String>,
    pub revised_date: Option<String>,
    pub revised_amount_minor: Option<i64>,
    pub revised_type: Option<String>,
    pub revised_description: Option<String>,
    pub manual_conflict: bool,
    pub original_source_row_reference: Option<String>,
    pub revised_source_row_reference: Option<String>,
}
struct RevisionCompareRow {
    id: String,
    date: Option<String>,
    amount_minor: Option<i64>,
    transaction_type: Option<String>,
    description: Option<String>,
    reference: Option<String>,
    allocations: Vec<(String, i64)>,
}
impl RevisionCompareRow {
    fn exact_match(&self, other: &Self) -> bool {
        self.date == other.date
            && self.amount_minor == other.amount_minor
            && self.transaction_type == other.transaction_type
            && normalize_description(self.description.as_deref())
                == normalize_description(other.description.as_deref())
            && self.allocations == other.allocations
    }

    fn reference_matches(&self, other: &Self) -> bool {
        self.reference
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .zip(
                other
                    .reference
                    .as_deref()
                    .filter(|value| !value.trim().is_empty()),
            )
            .is_some_and(|(left, right)| left == right)
    }

    fn same_financial_fields(&self, other: &Self) -> bool {
        self.date == other.date
            && self.amount_minor == other.amount_minor
            && self.transaction_type == other.transaction_type
            && self.allocations == other.allocations
    }

    fn description_matches(&self, other: &Self) -> bool {
        self.transaction_type == other.transaction_type
            && normalize_description(self.description.as_deref())
                .zip(normalize_description(other.description.as_deref()))
                .is_some_and(|(left, right)| left == right)
    }
}
struct RevisionChangeDraft {
    id: String,
    change_type: &'static str,
    original_record_id: Option<String>,
    revised_record_id: Option<String>,
}
impl RevisionChangeDraft {
    fn paired(
        change_type: &'static str,
        original: &RevisionCompareRow,
        revised: &RevisionCompareRow,
    ) -> Self {
        Self::single(change_type, Some(original), Some(revised))
    }

    fn single(
        change_type: &'static str,
        original: Option<&RevisionCompareRow>,
        revised: Option<&RevisionCompareRow>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            change_type,
            original_record_id: original.map(|row| row.id.clone()),
            revised_record_id: revised.map(|row| row.id.clone()),
        }
    }
}
#[derive(serde::Serialize, Debug)]
pub struct StagedImportRecord {
    pub id: String,
    pub source_row_reference: String,
    pub original_date: Option<String>,
    pub normalized_date: Option<String>,
    pub original_description: Option<String>,
    pub normalized_description: Option<String>,
    pub original_amount: Option<String>,
    pub normalized_amount_minor: Option<i64>,
    pub transaction_type: Option<String>,
    pub validation_status: String,
    pub duplicate_status: String,
    pub review_status: String,
    pub allocations: Vec<StagedCategoryAllocation>,
}
#[derive(serde::Serialize)]
pub struct StagedCategoryAllocationRecord {
    pub category_id: String,
    pub category_name: String,
    pub amount_minor: i64,
}
#[derive(serde::Serialize)]
pub struct ImportLogRecord {
    pub id: String,
    pub timestamp: String,
    pub stage: String,
    pub severity: String,
    pub message: String,
    pub record_id: Option<String>,
    pub corrective_action: Option<String>,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct ImportCategoryRuleRecord {
    pub category_id: String,
    pub category_name: String,
    pub category_type: String,
    pub match_text: String,
}
impl SqliteImportRepository<'_> {
    pub fn list_category_rules(&self) -> Result<Vec<ImportCategoryRuleRecord>, ImportError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut statement = connection
            .prepare(
                "SELECT r.category_id,c.name,r.category_type,r.match_text FROM import_category_rule_terms r JOIN categories c ON c.id=r.category_id WHERE c.archived_at IS NULL ORDER BY c.name COLLATE NOCASE,r.category_type,r.match_text COLLATE NOCASE",
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        statement
            .query_map([], |row| {
                Ok(ImportCategoryRuleRecord {
                    category_id: row.get(0)?,
                    category_name: row.get(1)?,
                    category_type: row.get(2)?,
                    match_text: row.get(3)?,
                })
            })
            .map_err(|error| ImportError::Database(error.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| ImportError::Database(error.to_string()))
    }

    pub fn save_category_rule(
        &self,
        category_id: Option<&str>,
        category_name: &str,
        category_type: &str,
        match_texts: &[String],
    ) -> Result<Vec<ImportCategoryRuleRecord>, ImportError> {
        let name = category_name.trim();
        if name.is_empty()
            || name.chars().count() > 100
            || !matches!(category_type, "income" | "expense")
            || match_texts.is_empty()
            || match_texts.len() > 50
        {
            return Err(ImportError::InvalidBatch);
        }
        let mut normalized = Vec::new();
        let mut unique = std::collections::HashSet::new();
        for value in match_texts {
            let term = value.trim();
            let key = term.to_lowercase();
            if term.is_empty() || term.chars().count() > 100 {
                return Err(ImportError::InvalidBatch);
            }
            if unique.insert(key) {
                normalized.push(term.to_string());
            }
        }
        if normalized.is_empty() {
            return Err(ImportError::InvalidBatch);
        }

        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let transaction = connection
            .transaction()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        let id = if let Some(category_id) = category_id {
            let active_type: Option<String> = transaction
                .query_row(
                    "SELECT category_type FROM categories WHERE id=?1 AND archived_at IS NULL",
                    [category_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| ImportError::Database(error.to_string()))?;
            if active_type.as_deref() != Some(category_type) {
                return Err(ImportError::InvalidBatch);
            }
            transaction
                .execute(
                    "UPDATE categories SET name=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                    params![name, category_id],
                )
                .map_err(|error| ImportError::Database(error.to_string()))?;
            category_id.to_string()
        } else {
            let existing: Option<String> = transaction
                .query_row(
                    "SELECT id FROM categories WHERE lower(name)=lower(?1) AND category_type=?2 AND archived_at IS NULL",
                    params![name, category_type],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| ImportError::Database(error.to_string()))?;
            if let Some(id) = existing {
                id
            } else {
                let id = Uuid::new_v4().to_string();
                transaction
                    .execute(
                        "INSERT INTO categories(id,name,category_type,is_system) VALUES (?1,?2,?3,0)",
                        params![id, name, category_type],
                    )
                    .map_err(|error| ImportError::Database(error.to_string()))?;
                id
            }
        };
        transaction
            .execute(
                "DELETE FROM import_category_rule_terms WHERE category_id=?1",
                [&id],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        for term in normalized {
            let conflict: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM import_category_rule_terms WHERE category_type=?1 AND lower(match_text)=lower(?2) AND category_id<>?3)",
                    params![category_type, term, id],
                    |row| row.get(0),
                )
                .map_err(|error| ImportError::Database(error.to_string()))?;
            if conflict {
                return Err(ImportError::InvalidBatch);
            }
            transaction
                .execute(
                    "INSERT INTO import_category_rule_terms(id,category_id,category_type,match_text) VALUES (?1,?2,?3,?4)",
                    params![Uuid::new_v4().to_string(), id, category_type, term],
                )
                .map_err(|error| ImportError::Database(error.to_string()))?;
        }
        transaction
            .commit()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        drop(connection);
        self.list_category_rules()
    }

    pub fn delete_category_rule(&self, category_id: &str) -> Result<(), ImportError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        connection
            .execute(
                "DELETE FROM import_category_rule_terms WHERE category_id=?1",
                [category_id],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        Ok(())
    }

    pub fn get_reconciliation(&self, batch_id: &str) -> Result<ImportReconciliation, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        refresh_source_retention(&c, batch_id)?;
        c.query_row("SELECT opening_balance_minor,closing_balance_minor,expected_closing_balance_minor,difference_minor,outcome,explanation FROM import_reconciliations WHERE batch_id=?1", [batch_id], |r|Ok(ImportReconciliation { opening_balance_minor:r.get(0)?, closing_balance_minor:r.get(1)?, expected_closing_balance_minor:r.get(2)?, difference_minor:r.get(3)?, outcome:r.get(4)?, explanation:r.get(5)? })).optional().map(|r|r.unwrap_or(ImportReconciliation{opening_balance_minor:None,closing_balance_minor:None,expected_closing_balance_minor:None,difference_minor:None,outcome:"unavailable".into(),explanation:"No statement balances provided.".into()})).map_err(|e|ImportError::Database(e.to_string()))
    }

    pub fn list_logs(&self, batch_id: &str) -> Result<Vec<ImportLogRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        refresh_source_retention(&c, batch_id)?;
        let mut s = c.prepare("SELECT id,timestamp,stage,severity,message,record_id,corrective_action FROM import_logs WHERE batch_id=?1 ORDER BY timestamp,id").map_err(|e| ImportError::Database(e.to_string()))?;
        s.query_map([batch_id], |r| {
            Ok(ImportLogRecord {
                id: r.get(0)?,
                timestamp: r.get(1)?,
                stage: r.get(2)?,
                severity: r.get(3)?,
                message: r.get(4)?,
                record_id: r.get(5)?,
                corrective_action: r.get(6)?,
            })
        })
        .map_err(|e| ImportError::Database(e.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ImportError::Database(e.to_string()))
    }

    pub fn list_records(&self, batch_id: &str) -> Result<Vec<StagedImportRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        refresh_source_retention(&c, batch_id)?;
        c.execute(
            "UPDATE imported_records AS candidate SET duplicate_status=CASE WHEN EXISTS(SELECT 1 FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN import_batches b ON b.id=candidate.batch_id JOIN accounts a ON a.id=b.account_id WHERE e.account_id=a.id AND t.status='posted' AND t.currency_code=a.currency_code AND t.amount_minor=candidate.normalized_amount_minor AND t.transaction_type=candidate.transaction_type AND date(t.occurred_at)=date(candidate.normalized_date) AND lower(trim(COALESCE(t.description,'')))=lower(trim(COALESCE(candidate.normalized_description,'')))) OR EXISTS(SELECT 1 FROM imported_records other WHERE other.batch_id=candidate.batch_id AND other.id<>candidate.id AND other.validation_status='valid' AND other.normalized_amount_minor=candidate.normalized_amount_minor AND other.transaction_type=candidate.transaction_type AND date(other.normalized_date)=date(candidate.normalized_date) AND lower(trim(COALESCE(other.normalized_description,'')))=lower(trim(COALESCE(candidate.normalized_description,'')))) THEN 'probable' ELSE 'unmatched' END WHERE candidate.batch_id=?1 AND candidate.validation_status='valid' AND candidate.transaction_type IN ('income','expense') AND candidate.review_status='pending' AND candidate.duplicate_status IN ('probable','unmatched') AND EXISTS(SELECT 1 FROM import_batches current_batch WHERE current_batch.id=candidate.batch_id AND current_batch.status='review' AND current_batch.revision_of_batch_id IS NULL)",
            [batch_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        let mut s=c.prepare("SELECT id,source_row_reference,original_date,normalized_date,original_description,normalized_description,original_amount,normalized_amount_minor,transaction_type,validation_status,duplicate_status,review_status FROM imported_records WHERE batch_id=?1 ORDER BY CAST(source_row_reference AS INTEGER)").map_err(|e|ImportError::Database(e.to_string()))?;
        let mut records = s
            .query_map([batch_id], |r| {
                Ok(StagedImportRecord {
                    id: r.get(0)?,
                    source_row_reference: r.get(1)?,
                    original_date: r.get(2)?,
                    normalized_date: r.get(3)?,
                    original_description: r.get(4)?,
                    normalized_description: r.get(5)?,
                    original_amount: r.get(6)?,
                    normalized_amount_minor: r.get(7)?,
                    transaction_type: r.get(8)?,
                    validation_status: r.get(9)?,
                    duplicate_status: r.get(10)?,
                    review_status: r.get(11)?,
                    allocations: Vec::new(),
                })
            })
            .map_err(|e| ImportError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        drop(s);
        drop(c);
        for record in &mut records {
            record.allocations = self
                .list_record_allocations(&record.id)?
                .into_iter()
                .map(|a| StagedCategoryAllocation {
                    category_id: a.category_id,
                    amount_minor: a.amount_minor,
                })
                .collect();
        }
        Ok(records)
    }

    pub fn list_record_allocations(
        &self,
        record_id: &str,
    ) -> Result<Vec<StagedCategoryAllocationRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut s = c.prepare("SELECT a.category_id,c.name,a.amount_minor FROM imported_record_allocations a JOIN categories c ON c.id=a.category_id WHERE a.imported_record_id=?1 ORDER BY c.name").map_err(|e|ImportError::Database(e.to_string()))?;
        s.query_map([record_id], |r| {
            Ok(StagedCategoryAllocationRecord {
                category_id: r.get(0)?,
                category_name: r.get(1)?,
                amount_minor: r.get(2)?,
            })
        })
        .map_err(|e| ImportError::Database(e.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ImportError::Database(e.to_string()))
    }
    pub fn stage_csv(
        &self,
        batch_id: &str,
        contents: &str,
        mapping: &CsvImportMapping,
    ) -> Result<usize, ImportError> {
        self.stage_csv_with_directions(batch_id, contents, mapping, None, None)
    }

    pub fn stage_csv_with_directions(
        &self,
        batch_id: &str,
        contents: &str,
        mapping: &CsvImportMapping,
        incoming_amount_column: Option<&str>,
        outgoing_amount_column: Option<&str>,
    ) -> Result<usize, ImportError> {
        if !matches!(
            mapping.positive_amount_type.as_str(),
            "income" | "expense" | "unknown"
        ) || !matches!(
            mapping.date_format.as_str(),
            "iso_date"
                | "rfc3339"
                | "day_month_year_slash"
                | "month_day_year_slash"
                | "day_month_year_dash"
        ) {
            return Err(ImportError::InvalidBatch);
        }
        let mut reader = csv::ReaderBuilder::new()
            .flexible(false)
            .delimiter(detect_csv_delimiter(contents.as_bytes()))
            .from_reader(contents.as_bytes());
        let headers = reader
            .headers()
            .map_err(|e| ImportError::Database(e.to_string()))?
            .clone();
        let date_index = headers
            .iter()
            .position(|h| h == mapping.date_column)
            .ok_or(ImportError::InvalidBatch)?;
        let amount_index = headers
            .iter()
            .position(|h| h == mapping.amount_column)
            .ok_or(ImportError::InvalidBatch)?;
        let incoming_index = incoming_amount_column
            .map(|column| {
                headers
                    .iter()
                    .position(|header| header == column)
                    .ok_or(ImportError::InvalidBatch)
            })
            .transpose()?;
        let outgoing_index = outgoing_amount_column
            .map(|column| {
                headers
                    .iter()
                    .position(|header| header == column)
                    .ok_or(ImportError::InvalidBatch)
            })
            .transpose()?;
        if incoming_index.is_some() && incoming_index == outgoing_index {
            return Err(ImportError::InvalidBatch);
        }
        let description_index = mapping
            .description_column
            .as_ref()
            .and_then(|column| headers.iter().position(|h| h == column));
        if mapping.description_column.is_some() && description_index.is_none() {
            return Err(ImportError::InvalidBatch);
        }
        let mut rows = Vec::new();
        for (index, row) in reader.records().enumerate() {
            let row = row.map_err(|e| ImportError::Database(e.to_string()))?;
            let raw_date = row.get(date_index).unwrap_or("").trim().to_string();
            let source_amount = row.get(amount_index).unwrap_or("").trim().to_string();
            let description = description_index
                .and_then(|i| row.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);
            let date = parse_import_date(&raw_date, &mapping.date_format);
            let (raw_amount, amount, transaction_type, status) = if incoming_index.is_some()
                || outgoing_index.is_some()
            {
                let incoming_raw = incoming_index
                    .and_then(|column| row.get(column))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let outgoing_raw = outgoing_index
                    .and_then(|column| row.get(column))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let incoming = parse_minor_units(&incoming_raw.replace(',', ""));
                let outgoing = parse_minor_units(&outgoing_raw.replace(',', ""));
                match (incoming, outgoing) {
                    (Some(_), Some(_)) => (
                        format!("in: {incoming_raw}; out: {outgoing_raw}"),
                        None,
                        None,
                        "uncertain",
                    ),
                    (Some(value), None) => (
                        incoming_raw,
                        Some(value),
                        Some("income"),
                        if date.is_some() { "valid" } else { "uncertain" },
                    ),
                    (None, Some(value)) => (
                        outgoing_raw,
                        Some(value),
                        Some("expense"),
                        if date.is_some() { "valid" } else { "uncertain" },
                    ),
                    (None, None) => (
                        source_amount,
                        None,
                        None,
                        if raw_date.is_empty() {
                            "invalid"
                        } else {
                            "uncertain"
                        },
                    ),
                }
            } else {
                let signed = source_amount.starts_with('-');
                let normalized = source_amount.replace(',', "");
                let amount = parse_minor_units(normalized.strip_prefix('-').unwrap_or(&normalized));
                let status = if raw_date.is_empty() || source_amount.is_empty() {
                    "invalid"
                } else if date.is_none() || amount.is_none() {
                    "uncertain"
                } else {
                    "valid"
                };
                let transaction_type = if signed {
                    Some(if mapping.positive_amount_type == "expense" {
                        "income"
                    } else {
                        "expense"
                    })
                } else if amount.is_some() && mapping.positive_amount_type != "unknown" {
                    Some(mapping.positive_amount_type.as_str())
                } else {
                    None
                };
                (source_amount, amount, transaction_type, status)
            };
            let status = if raw_date.is_empty() {
                "invalid"
            } else if date.is_none() || amount.is_none() || transaction_type.is_none() {
                "uncertain"
            } else {
                status
            };
            rows.push((
                index + 2,
                raw_date,
                date,
                description,
                raw_amount,
                amount,
                transaction_type.map(str::to_owned),
                status.to_string(),
            ));
        }
        if rows.is_empty() {
            return Err(ImportError::InvalidBatch);
        }
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = connection
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=?1 AND status='review' AND source_format IN ('csv','xlsx','pdf'))",[batch_id],|r|r.get(0)).map_err(|e|ImportError::Database(e.to_string()))?;
        if !exists {
            return Err(ImportError::InvalidBatch);
        }
        let already_staged: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM imported_records WHERE batch_id=?1)",
                [batch_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if already_staged {
            return Err(ImportError::InvalidBatch);
        }
        for (line, raw_date, date, description, raw_amount, amount, kind, status) in &rows {
            let duplicate = if status == "valid" {
                tx.query_row("SELECT EXISTS(SELECT 1 FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN import_batches b ON b.id=?1 JOIN accounts a ON a.id=b.account_id WHERE e.account_id=a.id AND t.status='posted' AND t.currency_code=a.currency_code AND t.amount_minor=?2 AND t.transaction_type=?3 AND date(t.occurred_at)=date(?4) AND lower(trim(COALESCE(t.description,'')))=lower(trim(COALESCE(?5,'')))) OR EXISTS(SELECT 1 FROM imported_records r WHERE r.batch_id=?1 AND r.validation_status='valid' AND r.normalized_amount_minor=?2 AND r.transaction_type=?3 AND date(r.normalized_date)=date(?4) AND lower(trim(COALESCE(r.normalized_description,'')))=lower(trim(COALESCE(?5,''))))",params![batch_id,amount,kind,date,description],|r|r.get::<_,bool>(0)).map_err(|e|ImportError::Database(e.to_string()))?
            } else {
                false
            };
            let duplicate_status = if duplicate { "probable" } else { "unmatched" };
            tx.execute("INSERT INTO imported_records(id,batch_id,source_row_reference,original_date,normalized_date,original_description,normalized_description,original_amount,normalized_amount_minor,currency_code,transaction_type,validation_status,duplicate_status) SELECT ?1,?2,?3,?4,?5,?6,?6,?7,?8,a.currency_code,?9,?10,?11 FROM import_batches b JOIN accounts a ON a.id=b.account_id WHERE b.id=?2",params![Uuid::new_v4().to_string(),batch_id,line.to_string(),raw_date,date,description,raw_amount,amount,kind,status,duplicate_status]).map_err(|e|ImportError::Database(e.to_string()))?;
        }
        tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'parse','INFO',?3)",params![Uuid::new_v4().to_string(),batch_id,format!("Staged {} CSV rows",rows.len())]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(rows.len())
    }

    pub fn xlsx_sheet_names(&self, contents: Vec<u8>) -> Result<Vec<String>, ImportError> {
        validate_workbook_expansion(&contents)?;
        let workbook = open_workbook_auto_from_rs(Cursor::new(contents))
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(workbook.sheet_names().to_vec())
    }

    pub fn stage_pdf_pages(
        &self,
        batch_id: &str,
        pages: Vec<String>,
    ) -> Result<usize, ImportError> {
        self.stage_pdf_pages_with_types(batch_id, pages, None)
    }

    pub fn stage_pdf_table_pages(
        &self,
        batch_id: &str,
        pages: Vec<String>,
        row_transaction_types: Vec<Option<String>>,
    ) -> Result<usize, ImportError> {
        self.stage_pdf_pages_with_types(batch_id, pages, Some(row_transaction_types))
    }

    fn stage_pdf_pages_with_types(
        &self,
        batch_id: &str,
        pages: Vec<String>,
        row_transaction_types: Option<Vec<Option<String>>>,
    ) -> Result<usize, ImportError> {
        let extracted_bytes = pages
            .iter()
            .try_fold(0usize, |total, page| total.checked_add(page.len()))
            .ok_or(ImportError::ExtractedTextTooLarge)?;
        if extracted_bytes as u64 > MAX_WORKBOOK_EXPANDED_BYTES {
            return Err(ImportError::ExtractedTextTooLarge);
        }
        let mut extracted_rows = Vec::new();
        for (page_index, page) in pages.iter().enumerate() {
            for (line_index, line) in page.lines().enumerate() {
                let text = line.split_whitespace().collect::<Vec<_>>().join(" ");
                if !text.is_empty() {
                    let candidate_date = extract_pdf_date(&text).unwrap_or_default();
                    let candidate_amount =
                        extract_pdf_amount(&text, row_transaction_types.is_some())
                            .unwrap_or_default();
                    extracted_rows.push((
                        format!(
                            "{:08} — Page {}, line {}",
                            extracted_rows.len() + 1,
                            page_index + 1,
                            line_index + 1
                        ),
                        text,
                        candidate_date,
                        candidate_amount,
                    ));
                }
            }
        }
        if extracted_rows.is_empty() {
            return Err(ImportError::PdfNoExtractableText);
        }
        if let Some(types) = &row_transaction_types {
            if types.len() != extracted_rows.len()
                || types
                    .iter()
                    .flatten()
                    .any(|kind| !matches!(kind.as_str(), "income" | "expense"))
            {
                return Err(ImportError::InvalidBatch);
            }
        }
        let mut csv_bytes = Vec::new();
        {
            let mut writer = csv::Writer::from_writer(&mut csv_bytes);
            writer
                .write_record(["Date", "Description", "Amount"])
                .map_err(|e| ImportError::Database(e.to_string()))?;
            for (_, text, date, amount) in &extracted_rows {
                writer
                    .write_record([date, text, amount])
                    .map_err(|e| ImportError::Database(e.to_string()))?;
            }
            writer
                .flush()
                .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Description".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        let csv =
            std::str::from_utf8(&csv_bytes).map_err(|e| ImportError::Database(e.to_string()))?;
        let count = self.stage_csv(batch_id, csv, &mapping)?;
        if count != extracted_rows.len() {
            return Err(ImportError::InvalidBatch);
        }
        let records = self.list_records(batch_id)?;
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = connection
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        for (index, (record, (reference, _, _, _))) in
            records.iter().zip(extracted_rows).enumerate()
        {
            let inferred_type = row_transaction_types
                .as_ref()
                .and_then(|types| types[index].as_deref());
            if record.validation_status == "valid" {
                if let Some(kind) = inferred_type {
                    let amount = record
                        .normalized_amount_minor
                        .ok_or(ImportError::InvalidBatch)?;
                    let date = record
                        .normalized_date
                        .as_deref()
                        .ok_or(ImportError::InvalidBatch)?;
                    let duplicate: bool = tx
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN import_batches b ON b.id=?1 JOIN accounts a ON a.id=b.account_id WHERE e.account_id=a.id AND t.status='posted' AND t.currency_code=a.currency_code AND t.amount_minor=?2 AND t.transaction_type=?3 AND date(t.occurred_at)=date(?4) AND lower(trim(COALESCE(t.description,'')))=lower(trim(COALESCE(?6,'')))) OR EXISTS(SELECT 1 FROM imported_records r WHERE r.batch_id=?1 AND r.id<>?5 AND r.validation_status='valid' AND r.normalized_amount_minor=?2 AND r.transaction_type=?3 AND date(r.normalized_date)=date(?4) AND lower(trim(COALESCE(r.normalized_description,'')))=lower(trim(COALESCE(?6,''))))",
                            params![batch_id, amount, kind, date, record.id, record.normalized_description],
                            |r| r.get(0),
                        )
                        .map_err(|e| ImportError::Database(e.to_string()))?;
                    tx.execute(
                        "UPDATE imported_records SET source_row_reference=?1,transaction_type=?2,validation_status='valid',duplicate_status=?3 WHERE id=?4",
                        params![reference,kind,if duplicate {"probable"} else {"unmatched"},record.id],
                    )
                    .map_err(|e| ImportError::Database(e.to_string()))?;
                    continue;
                }
            }
            tx.execute(
                "UPDATE imported_records SET source_row_reference=?1,validation_status='uncertain',transaction_type=NULL WHERE id=?2",
                params![reference, record.id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        let (severity, message, corrective_action) = if row_transaction_types.is_some() {
            (
                "WARNING",
                "Extracted PDF table rows for review; no values were posted automatically",
                "Rows with a clear date, amount, and transaction direction can be confirmed together. Rows with ambiguous status or direction remain uncertain and need individual review.",
            )
        } else {
            (
                "WARNING",
                "Extracted PDF text into uncertain review rows; no values were posted automatically",
                "Review every extracted row before confirmation. Generic PDF layouts may include headings or other non-transaction text; correct or skip those rows.",
            )
        };
        tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message,corrective_action) VALUES (?1,?2,'parse',?3,?4,?5)",params![Uuid::new_v4().to_string(),batch_id,severity,message,corrective_action]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(count)
    }

    pub fn xlsx_headers(
        &self,
        contents: Vec<u8>,
        sheet_name: &str,
        header_row: usize,
    ) -> Result<Vec<String>, ImportError> {
        validate_workbook_expansion(&contents)?;
        if header_row == 0 {
            return Err(ImportError::InvalidBatch);
        }
        let mut workbook = open_workbook_auto_from_rs(Cursor::new(contents))
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let row = range
            .rows()
            .nth(header_row - 1)
            .ok_or(ImportError::InvalidBatch)?;
        Ok(row
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let value = cell_string(cell);
                if value.trim().is_empty() {
                    format!("Column {}", index + 1)
                } else {
                    value.trim().to_string()
                }
            })
            .collect())
    }

    pub fn xlsx_rows(
        &self,
        contents: Vec<u8>,
        sheet_name: &str,
        header_row: usize,
    ) -> Result<Vec<Vec<String>>, ImportError> {
        validate_workbook_expansion(&contents)?;
        if header_row == 0 {
            return Err(ImportError::InvalidBatch);
        }
        let mut workbook = open_workbook_auto_from_rs(Cursor::new(contents))
            .map_err(|error| ImportError::Database(error.to_string()))?;
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|error| ImportError::Database(error.to_string()))?;
        Ok(range
            .rows()
            .skip(header_row - 1)
            .map(|row| row.iter().map(cell_string).collect())
            .collect())
    }

    pub fn stage_xlsx(
        &self,
        batch_id: &str,
        contents: Vec<u8>,
        sheet_name: &str,
        header_row: usize,
        mapping: &CsvImportMapping,
    ) -> Result<usize, ImportError> {
        self.stage_xlsx_with_directions(
            batch_id, contents, sheet_name, header_row, mapping, None, None,
        )
    }

    pub fn stage_xlsx_with_directions(
        &self,
        batch_id: &str,
        contents: Vec<u8>,
        sheet_name: &str,
        header_row: usize,
        mapping: &CsvImportMapping,
        incoming_amount_column: Option<&str>,
        outgoing_amount_column: Option<&str>,
    ) -> Result<usize, ImportError> {
        validate_workbook_expansion(&contents)?;
        if header_row == 0 {
            return Err(ImportError::InvalidBatch);
        }
        let mut workbook = open_workbook_auto_from_rs(Cursor::new(contents))
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let formula_range = workbook
            .worksheet_formula(sheet_name)
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let rows: Vec<_> = range.rows().collect();
        let headers = rows.get(header_row - 1).ok_or(ImportError::InvalidBatch)?;
        let header_names: Vec<String> = headers
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let v = cell_string(c);
                if v.trim().is_empty() {
                    format!("Column {}", i + 1)
                } else {
                    v.trim().to_string()
                }
            })
            .collect();
        let date_index = header_names
            .iter()
            .position(|h| h == &mapping.date_column)
            .ok_or(ImportError::InvalidBatch)?;
        let amount_index = header_names
            .iter()
            .position(|h| h == &mapping.amount_column)
            .ok_or(ImportError::InvalidBatch)?;
        let incoming_index = incoming_amount_column
            .map(|column| {
                header_names
                    .iter()
                    .position(|header| header == column)
                    .ok_or(ImportError::InvalidBatch)
            })
            .transpose()?;
        let outgoing_index = outgoing_amount_column
            .map(|column| {
                header_names
                    .iter()
                    .position(|header| header == column)
                    .ok_or(ImportError::InvalidBatch)
            })
            .transpose()?;
        if incoming_index.is_some() && incoming_index == outgoing_index {
            return Err(ImportError::InvalidBatch);
        }
        if let Some(name) = &mapping.description_column {
            if !header_names.iter().any(|h| h == name) {
                return Err(ImportError::InvalidBatch);
            }
        }
        let mut source_rows = Vec::new();
        let mut formula_rows = 0usize;
        let mut csv_bytes = Vec::new();
        {
            let mut writer = csv::Writer::from_writer(&mut csv_bytes);
            let mut csv_headers = vec!["__etracker_source_row".to_string()];
            csv_headers.extend(header_names.clone());
            writer
                .write_record(&csv_headers)
                .map_err(|e| ImportError::Database(e.to_string()))?;
            let (start_row, start_column) = range
                .start()
                .map(|(row, col)| (row as usize, col as usize))
                .unwrap_or((0, 0));
            for (index, row) in rows.iter().enumerate().skip(header_row) {
                let source_row = start_row + index + 1;
                let date_formula = formula_range
                    .get_value((
                        start_row as u32 + index as u32,
                        start_column as u32 + date_index as u32,
                    ))
                    .is_some_and(|formula| !formula.is_empty());
                let amount_formula = [Some(amount_index), incoming_index, outgoing_index]
                    .into_iter()
                    .flatten()
                    .any(|column| {
                        formula_range
                            .get_value((
                                start_row as u32 + index as u32,
                                start_column as u32 + column as u32,
                            ))
                            .is_some_and(|formula| !formula.is_empty())
                    });
                let mut values = Vec::with_capacity(header_names.len() + 1);
                values.push(format!("{}!{}", sheet_name, source_row));
                for column in 0..header_names.len() {
                    let formula = (column == date_index && date_formula)
                        || ([Some(amount_index), incoming_index, outgoing_index]
                            .into_iter()
                            .flatten()
                            .any(|mapped| column == mapped)
                            && amount_formula);
                    if formula {
                        values.push(String::new());
                    } else {
                        values.push(row.get(column).map(cell_string).unwrap_or_default());
                    }
                }
                if date_formula || amount_formula {
                    formula_rows += 1;
                }
                source_rows.push(format!("{}!{}", sheet_name, source_row));
                writer
                    .write_record(values)
                    .map_err(|e| ImportError::Database(e.to_string()))?;
            }
            writer
                .flush()
                .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        let mut csv_mapping = mapping.clone();
        csv_mapping.date_column = mapping.date_column.clone();
        let count = self.stage_csv_with_directions(
            batch_id,
            std::str::from_utf8(&csv_bytes).map_err(|e| ImportError::Database(e.to_string()))?,
            &csv_mapping,
            incoming_amount_column,
            outgoing_amount_column,
        )?;
        let records = self.list_records(batch_id)?;
        if records.len() != source_rows.len() || records.len() != count {
            return Err(ImportError::InvalidBatch);
        }
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        for (record, source_row) in records.iter().zip(source_rows) {
            tx.execute(
                "UPDATE imported_records SET source_row_reference=?1 WHERE id=?2",
                params![source_row, record.id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        if formula_rows > 0 {
            tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message,corrective_action) VALUES (?1,?2,'parse','WARNING',?3,'Correct formula-derived date or amount cells after reviewing the original worksheet.')",params![Uuid::new_v4().to_string(),batch_id,format!("{} rows with formulas in mapped date or amount columns were staged as invalid for review",formula_rows)]).map_err(|e|ImportError::Database(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(count)
    }
    pub fn list_batches(&self) -> Result<Vec<ImportBatchRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut s=c.prepare("SELECT id,account_id,source_file_name,source_format,CASE WHEN discarded_at IS NOT NULL THEN 'discarded' ELSE status END,reconciliation_status,created_at,revision_of_batch_id FROM import_batches ORDER BY created_at DESC").map_err(|e|ImportError::Database(e.to_string()))?;
        s.query_map([], |r| {
            Ok(ImportBatchRecord {
                id: r.get(0)?,
                account_id: r.get(1)?,
                source_file_name: r.get(2)?,
                source_format: r.get(3)?,
                status: r.get(4)?,
                reconciliation_status: r.get(5)?,
                created_at: r.get(6)?,
                revision_of_batch_id: r.get(7)?,
            })
        })
        .map_err(|e| ImportError::Database(e.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ImportError::Database(e.to_string()))
    }

    pub fn discard_batch(&self, batch_id: &str) -> Result<(), ImportError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = connection
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let changed = tx
            .execute(
                "UPDATE import_batches SET discarded_at=CURRENT_TIMESTAMP WHERE id=?1 AND status='review' AND discarded_at IS NULL",
                [batch_id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if changed != 1 {
            return Err(ImportError::InvalidBatch);
        }
        tx.execute(
            "INSERT INTO import_logs(id,batch_id,stage,severity,message,corrective_action) VALUES (?1,?2,'review','INFO','Import workspace discarded','The workspace is read-only; its source file remains available during the retention period.')",
            params![Uuid::new_v4().to_string(), batch_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))
    }
}
fn calculate_reconciliation(
    tx: &Transaction<'_>,
    batch_id: &str,
    opening: Option<i64>,
    closing: Option<i64>,
) -> Result<ImportReconciliation, ImportError> {
    let (Some(opening), Some(closing)) = (opening, closing) else {
        return Ok(ImportReconciliation {
            opening_balance_minor: None,
            closing_balance_minor: None,
            expected_closing_balance_minor: None,
            difference_minor: None,
            outcome: "unavailable".into(),
            explanation: "Statement opening or closing balance was not provided.".into(),
        });
    };
    let invalid: i64 = tx.query_row("SELECT COUNT(*) FROM imported_records WHERE batch_id=?1 AND validation_status<>'valid'", [batch_id], |r| r.get(0)).map_err(|e| ImportError::Database(e.to_string()))?;
    if invalid > 0 {
        return Err(ImportError::InvalidBatch);
    }
    let mut stmt = tx.prepare("SELECT normalized_amount_minor,transaction_type FROM imported_records WHERE batch_id=?1 AND validation_status='valid'").map_err(|e|ImportError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([batch_id], |r| {
            Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| ImportError::Database(e.to_string()))?;
    let mut movement = 0i64;
    let mut total = 0usize;
    for row in rows {
        let (amount, kind) = row.map_err(|e| ImportError::Database(e.to_string()))?;
        let (Some(amount), Some(kind)) = (amount, kind) else {
            return Err(ImportError::InvalidBatch);
        };
        if amount <= 0 {
            return Err(ImportError::InvalidBatch);
        }
        let signed = match kind.as_str() {
            "income" => amount,
            "expense" => -amount,
            _ => return Err(ImportError::InvalidBatch),
        };
        movement = movement
            .checked_add(signed)
            .ok_or(ImportError::InvalidBatch)?;
        total += 1;
    }
    if total == 0 {
        return Err(ImportError::InvalidBatch);
    }
    let expected = opening
        .checked_add(movement)
        .ok_or(ImportError::InvalidBatch)?;
    let difference = closing
        .checked_sub(expected)
        .ok_or(ImportError::InvalidBatch)?;
    let outcome = if difference == 0 {
        "reconciled"
    } else {
        "discrepancy"
    };
    Ok(ImportReconciliation {
        opening_balance_minor: Some(opening),
        closing_balance_minor: Some(closing),
        expected_closing_balance_minor: Some(expected),
        difference_minor: Some(difference),
        outcome: outcome.into(),
        explanation: if difference == 0 {
            "Statement closing balance matches the opening balance plus the staged movements."
                .into()
        } else {
            "Statement closing balance differs from the calculated balance. Review the staged rows and statement conventions.".into()
        },
    })
}
fn load_revision_rows(
    tx: &Transaction<'_>,
    batch_id: &str,
) -> Result<Vec<RevisionCompareRow>, ImportError> {
    let mut statement = tx
        .prepare("SELECT id,normalized_date,normalized_amount_minor,transaction_type,normalized_description,transaction_reference FROM imported_records WHERE batch_id=?1 ORDER BY source_row_reference")
        .map_err(|e| ImportError::Database(e.to_string()))?;
    let mut rows = statement
        .query_map([batch_id], |row| {
            Ok(RevisionCompareRow {
                id: row.get(0)?,
                date: row.get(1)?,
                amount_minor: row.get(2)?,
                transaction_type: row.get(3)?,
                description: row.get(4)?,
                reference: row.get(5)?,
                allocations: Vec::new(),
            })
        })
        .map_err(|e| ImportError::Database(e.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ImportError::Database(e.to_string()))?;
    drop(statement);
    for row in &mut rows {
        let mut allocations = tx
            .prepare("SELECT category_id,amount_minor FROM imported_record_allocations WHERE imported_record_id=?1 ORDER BY category_id")
            .map_err(|e| ImportError::Database(e.to_string()))?;
        row.allocations = allocations
            .query_map([&row.id], |allocation| {
                Ok((allocation.get(0)?, allocation.get(1)?))
            })
            .map_err(|e| ImportError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ImportError::Database(e.to_string()))?;
    }
    Ok(rows)
}
fn normalize_description(value: Option<&str>) -> Option<String> {
    value
        .map(|text| {
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|text| !text.is_empty())
}
fn cell_string(cell: &Data) -> String {
    match cell {
        Data::Empty | Data::Error(_) => String::new(),
        _ if cell.is_datetime() => cell
            .as_date()
            .map(|date| date.format("%Y-%m-%d").to_string())
            .or_else(|| {
                cell.as_datetime()
                    .map(|date| date.format("%Y-%m-%dT%H:%M:%S").to_string())
            })
            .unwrap_or_default(),
        _ => cell.to_string(),
    }
}
fn refresh_source_retention(
    connection: &rusqlite::Connection,
    batch_id: &str,
) -> Result<(), ImportError> {
    connection
        .execute(
            "UPDATE import_source_files SET expires_at=datetime('now', '+90 days') WHERE batch_id=?1 AND expires_at>CURRENT_TIMESTAMP",
            [batch_id],
        )
        .map(|_| ())
        .map_err(|e| ImportError::Database(e.to_string()))
}
fn parse_minor_units(raw: &str) -> Option<i64> {
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let major = whole.parse::<i64>().ok()?;
    let cents = if fraction.is_empty() {
        0
    } else if fraction.len() == 1 {
        fraction.parse::<i64>().ok()? * 10
    } else {
        fraction.parse::<i64>().ok()?
    };
    let amount = major.checked_mul(100)?.checked_add(cents)?;
    (amount > 0).then_some(amount)
}

fn extract_pdf_date(text: &str) -> Option<String> {
    let tokens = text
        .split_whitespace()
        .map(|token| {
            token.trim_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '-' && character != '/'
            })
        })
        .collect::<Vec<_>>();
    for token in &tokens {
        for format in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d/%m/%y", "%d-%m-%y"] {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(token, format) {
                return Some(date.to_string());
            }
        }
    }
    for words in tokens.windows(3) {
        let candidate = words.join(" ");
        for format in ["%d %b %Y", "%d %B %Y", "%b %d %Y", "%B %d %Y"] {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&candidate, format) {
                return Some(date.to_string());
            }
        }
    }
    None
}

fn extract_pdf_amount(text: &str, allow_trailing_integer: bool) -> Option<String> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate().rev() {
        let clean = token.trim_matches(|character: char| {
            !character.is_ascii_alphanumeric()
                && !matches!(character, '.' | ',' | '-' | '(' | ')' | '$' | '£' | '€')
        });
        let upper = clean.to_ascii_uppercase();
        let has_currency = ["KES", "KSH", "USD", "GBP", "EUR", "$", "£", "€"]
            .iter()
            .any(|currency| upper.starts_with(currency));
        let mut number = clean.to_string();
        for prefix in ["KES", "KSH", "USD", "GBP", "EUR", "$", "£", "€"] {
            if number.to_ascii_uppercase().starts_with(prefix) {
                number = number[prefix.len()..].to_string();
                break;
            }
        }
        let negative = number.starts_with('-') || number.starts_with('(');
        number = number
            .trim_start_matches('-')
            .trim_start_matches('(')
            .to_string();
        number = number.trim_end_matches(')').replace(',', "");
        if !number.contains('.')
            && !has_currency
            && !(allow_trailing_integer && index + 1 == tokens.len())
        {
            continue;
        }
        if parse_minor_units(&number).is_some() {
            return Some(if negative {
                format!("-{number}")
            } else {
                number
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::Database;
    use std::io::{Cursor, Write};
    use std::path::Path;

    #[test]
    fn parses_explicit_statement_date_formats_without_ambiguous_guessing() {
        assert_eq!(
            parse_import_date("06/11/2026", "day_month_year_slash").as_deref(),
            Some("2026-11-06T00:00:00Z")
        );
        assert_eq!(
            parse_import_date("06/11/2026", "month_day_year_slash").as_deref(),
            Some("2026-06-11T00:00:00Z")
        );
        assert_eq!(
            parse_import_date("06-11-2026", "day_month_year_dash").as_deref(),
            Some("2026-11-06T00:00:00Z")
        );
        assert_eq!(
            parse_import_date("31/11/2026", "day_month_year_slash"),
            None
        );
        assert_eq!(parse_import_date("06/11/2026", "unknown"), None);
    }

    #[test]
    fn workbook_expansion_limit_accepts_boundary_and_rejects_excess_or_overflow() {
        assert!(
            check_workbook_expanded_size([Ok(MAX_WORKBOOK_EXPANDED_BYTES - 1), Ok(1),]).is_ok()
        );
        assert!(matches!(
            check_workbook_expanded_size([Ok(MAX_WORKBOOK_EXPANDED_BYTES), Ok(1)]),
            Err(ImportError::WorkbookTooLarge)
        ));
        assert!(matches!(
            check_workbook_expanded_size([Ok(u64::MAX), Ok(1)]),
            Err(ImportError::WorkbookTooLarge)
        ));
    }

    #[test]
    fn extracts_common_pdf_date_and_signed_amount_candidates_without_inference() {
        assert_eq!(
            extract_pdf_date("Purchase 06/11/2026 -1,234.50"),
            Some("2026-11-06".into())
        );
        assert_eq!(
            extract_pdf_amount("Purchase 06/11/2026 -1,234.50", false),
            Some("-1234.50".into())
        );
        assert_eq!(
            extract_pdf_amount("Transfer reference 2026-11-06", false),
            None
        );
        assert_eq!(
            extract_pdf_amount("Purchase 06/11/2026 15", true),
            Some("15".into())
        );
    }

    fn db() -> Database {
        Database::initialize(Path::new(":memory:")).unwrap()
    }
    fn workbook_fixture() -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let files = [
            (
                "[Content_Types].xml",
                r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
            ),
            (
                "_rels/.rels",
                r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Transactions" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Date</t></is></c><c r="B1" t="inlineStr"><is><t>Amount</t></is></c><c r="C1" t="inlineStr"><is><t>Memo</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>2026-11-05</t></is></c><c r="B2"><v>12.5</v></c><c r="C2" t="inlineStr"><is><t>Market</t></is></c></row><row r="3"><c r="A3" t="inlineStr"><is><t>2026-11-06</t></is></c><c r="B3"><f>SUM(B2:B2)</f><v>12.5</v></c><c r="C3" t="inlineStr"><is><t>Calculated total</t></is></c></row></sheetData></worksheet>"#,
            ),
        ];
        for (name, contents) in files {
            zip.start_file(name, options).unwrap();
            zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    fn account(db: &Database) {
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO accounts(id,name,account_type) VALUES ('a','Main','cash')",
                [],
            )
            .unwrap();
    }
    #[test]
    fn persists_batch_for_resume_without_ledger_entries() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let batch = NewImportBatch {
            account_id: "a".into(),
            source_file_name: "statement.csv".into(),
            source_format: "csv".into(),
            revision_of_batch_id: None,
            source_file_contents: b"statement fixture".to_vec(),
        };
        let id = repository.create_batch(&batch).unwrap();
        assert_eq!(repository.list_batches().unwrap().len(), 1);
        let stored_source = repository.source_file_contents(&id).unwrap();
        let c = db.connection().lock().unwrap();
        let entries: i64 = c
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        let logs: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM import_logs WHERE batch_id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        let stored_hash: String = c
            .query_row(
                "SELECT source_file_hash FROM import_batches WHERE id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(entries, 0);
        assert_eq!(logs, 1);
        assert_eq!(stored_source, b"statement fixture");
        assert_eq!(
            stored_hash,
            format!("{:x}", Sha256::digest(b"statement fixture"))
        );
    }

    #[test]
    fn discarding_workspace_makes_it_read_only_and_retains_source_for_cleanup_period() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let id = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();

        repository.discard_batch(&id).unwrap();
        assert_eq!(repository.list_batches().unwrap()[0].status, "discarded");
        assert_eq!(
            repository.source_file_contents(&id).unwrap(),
            b"statement fixture"
        );
        assert!(repository.discard_batch(&id).is_err());

        let connection = db.connection().lock().unwrap();
        let blocked = connection.execute(
            "INSERT INTO imported_records(id,batch_id,source_row_reference,original_date,original_description,original_amount,currency_code,validation_status) VALUES ('discarded-row',?1,'2','2026-11-01','Lunch','10.00','KES','uncertain')",
            [&id],
        );
        assert!(blocked.is_err());
    }

    #[test]
    fn source_retention_expires_and_activity_refreshes_it() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let id = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE import_source_files SET expires_at=datetime('now', '-1 day') WHERE batch_id=?1",
                [&id],
            )
            .unwrap();
        assert!(matches!(
            repository.source_file_contents(&id),
            Err(ImportError::InvalidBatch)
        ));
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES ('activity',?1,'review','INFO','Reviewed import')",
                [&id],
            )
            .unwrap();
        assert_eq!(
            repository.source_file_contents(&id).unwrap(),
            b"statement fixture"
        );
    }
    #[test]
    fn rejects_archived_account() {
        let db = db();
        account(&db);
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE accounts SET archived_at=CURRENT_TIMESTAMP WHERE id='a'",
                [],
            )
            .unwrap();
        let repo = SqliteImportRepository::new(&db);
        let batch = NewImportBatch {
            account_id: "a".into(),
            source_file_name: "statement.csv".into(),
            source_format: "csv".into(),
            revision_of_batch_id: None,
            source_file_contents: b"statement fixture".to_vec(),
        };
        assert!(matches!(
            repo.create_batch(&batch),
            Err(ImportError::InvalidBatch)
        ));
    }
    #[test]
    fn csv_staging_preserves_quoted_fields_and_marks_bad_dates_uncertain() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let id = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "mapped.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let csv = "Date,Description,Amount\n2026-11-01,\"Cafe, downtown\",12.50\nnot-a-date,Unknown,4.00\n";
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Description".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        assert_eq!(repo.stage_csv(&id, csv, &mapping).unwrap(), 2);
        let rows = repo.list_records(&id).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].original_description.as_deref(),
            Some("Cafe, downtown")
        );
        assert_eq!(rows[0].normalized_amount_minor, Some(1250));
        assert_eq!(rows[1].validation_status, "uncertain");
        let entries: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entries, 0);
    }

    #[test]
    fn csv_staging_uses_detected_semicolon_delimiter_and_explicit_date_format() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let id = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "semicolon.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Memo".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "day_month_year_slash".into(),
        };
        assert_eq!(
            repo.stage_csv(
                &id,
                "Date;Amount;Memo\n06/11/2026;12.50;\"Cafe, downtown\"\n",
                &mapping
            )
            .unwrap(),
            1
        );
        let records = repo.list_records(&id).unwrap();
        assert_eq!(records[0].validation_status, "valid");
        assert_eq!(
            records[0].normalized_date.as_deref(),
            Some("2026-11-06T00:00:00Z")
        );
        assert_eq!(
            records[0].normalized_description.as_deref(),
            Some("Cafe, downtown")
        );
    }
    #[test]
    fn corrected_csv_record_stays_staged_and_is_audited() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "d".into(),
            description_column: None,
            amount_column: "a".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(&batch, "d,a\nbad,1.00\n", &mapping).unwrap();
        let row = repo.list_records(&batch).unwrap().remove(0);
        repo.correct_record(
            &row.id,
            "2026-11-06T00:00:00Z",
            100,
            "income",
            Some("Corrected"),
        )
        .unwrap();
        let corrected = repo.list_records(&batch).unwrap().remove(0);
        assert_eq!(corrected.validation_status, "valid");
        assert_eq!(corrected.review_status, "corrected");
        assert_eq!(
            corrected.normalized_description.as_deref(),
            Some("Corrected")
        );
        let logs: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM import_logs WHERE batch_id=?1 AND stage='review'",
                [&batch],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(logs, 1);
    }

    #[test]
    fn correcting_into_existing_transaction_requires_duplicate_review() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "d".into(),
            description_column: None,
            amount_column: "a".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(&batch, "d,a\nbad,1.00\n", &mapping).unwrap();
        let row = repo.list_records(&batch).unwrap().remove(0);
        {
            let c = db.connection().lock().unwrap();
            c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('existing','expense',1250,'KES','2026-11-06T00:00:00Z','manual')", []).unwrap();
            c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('existing-entry','existing','a',-1250)", []).unwrap();
        }
        repo.correct_record(&row.id, "2026-11-06T00:00:00Z", 1250, "expense", None)
            .unwrap();
        let corrected = repo.list_records(&batch).unwrap().remove(0);
        assert_eq!(corrected.duplicate_status, "probable");
        assert_eq!(corrected.review_status, "pending");
        assert!(matches!(
            repo.finalize(&batch),
            Err(ImportError::InvalidBatch)
        ));
    }
    #[test]
    fn detects_multifactor_duplicate_candidate_without_auto_discard() {
        let db = db();
        account(&db);
        {
            let c = db.connection().lock().unwrap();
            c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('existing','expense',1250,'KES','2026-11-01T00:00:00Z','manual')",[]).unwrap();
            c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('entry','existing','a',-1250)",[]).unwrap();
        }
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "d".into(),
            description_column: None,
            amount_column: "a".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(&batch, "d,a\n2026-11-01,12.50\n", &mapping)
            .unwrap();
        let row = repo.list_records(&batch).unwrap().remove(0);
        assert_eq!(row.duplicate_status, "probable");
        assert_eq!(row.review_status, "pending");
        let count: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    fn staged_expense(repo: &SqliteImportRepository<'_>, batch: &str) -> String {
        let mapping = CsvImportMapping {
            date_column: "d".into(),
            description_column: Some("memo".into()),
            amount_column: "a".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(batch, "d,memo,a\n2026-11-05,Groceries,12.50\n", &mapping)
            .unwrap();
        repo.list_records(batch).unwrap().remove(0).id
    }

    #[test]
    fn compares_revised_statement_additions_removals_and_modified_manual_records() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let original_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "original.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"original statement".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Memo".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repository
            .stage_csv(
                &original_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,12.50\n2026-11-06,Transport,5.00\n",
                &mapping,
            )
            .unwrap();
        let original_records = repository.list_records(&original_batch).unwrap();
        for record in &original_records {
            repository.review_record(&record.id, true).unwrap();
        }
        repository.finalize(&original_batch).unwrap();
        let original_transaction_id: String = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT linked_transaction_id FROM imported_records WHERE id=?1",
                [&original_records[0].id],
                |row| row.get(0),
            )
            .unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO transaction_audit_log(id,transaction_id,action) VALUES ('manual-edit',?1,'updated')",
                [original_transaction_id],
            )
            .unwrap();
        let revised_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "revised.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: Some(original_batch),
                source_file_contents: b"revised statement".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &revised_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,15.00\n2026-11-07,Coffee,3.50\n",
                &mapping,
            )
            .unwrap();
        let changes = repository.compare_revision(&revised_batch).unwrap();
        assert_eq!(changes.len(), 3);
        let modified = changes
            .iter()
            .find(|change| change.change_type == "modified")
            .unwrap();
        assert!(modified.manual_conflict);
        assert_eq!(modified.original_amount_minor, Some(1250));
        assert_eq!(modified.revised_amount_minor, Some(1500));
        assert!(changes.iter().any(|change| change.change_type == "removed"));
        assert!(changes.iter().any(|change| change.change_type == "added"));
    }

    #[test]
    fn competing_identical_revision_rows_are_left_ambiguous_for_manual_review() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Memo".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        let original_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "original.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"original".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &original_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,12.50\n2026-11-05,Groceries,12.50\n",
                &mapping,
            )
            .unwrap();
        for record in repository.list_records(&original_batch).unwrap() {
            repository.review_record(&record.id, true).unwrap();
        }
        repository.finalize(&original_batch).unwrap();
        let revised_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "revised.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: Some(original_batch),
                source_file_contents: b"revised".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &revised_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,12.50\n",
                &mapping,
            )
            .unwrap();
        let changes = repository.compare_revision(&revised_batch).unwrap();
        assert_eq!(changes.len(), 3);
        assert!(
            changes
                .iter()
                .all(|change| change.change_type == "ambiguous")
        );
        assert!(changes.iter().all(|change| change.resolution.is_none()));
    }

    #[test]
    fn explicitly_replacing_a_revised_manual_record_creates_linked_reversal_atomically() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Memo".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        let original_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "original.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"original statement".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &original_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,12.50\n",
                &mapping,
            )
            .unwrap();
        let old_record = repository.list_records(&original_batch).unwrap().remove(0);
        repository.review_record(&old_record.id, true).unwrap();
        repository.finalize(&original_batch).unwrap();
        let old_transaction: String = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT linked_transaction_id FROM imported_records WHERE id=?1",
                [&old_record.id],
                |row| row.get(0),
            )
            .unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO transaction_audit_log(id,transaction_id,action,previous_values,new_values) VALUES ('manual-change',?1,'updated','{}','{}')",
                [&old_transaction],
            )
            .unwrap();
        let revised_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "revised.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: Some(original_batch),
                source_file_contents: b"revised statement".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &revised_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,15.00\n",
                &mapping,
            )
            .unwrap();
        let change = repository
            .compare_revision(&revised_batch)
            .unwrap()
            .remove(0);
        assert!(change.manual_conflict);
        repository
            .set_revision_resolution(&change.id, "use_revised")
            .unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_revision_replacement BEFORE INSERT ON financial_transactions WHEN NEW.source_type='import' AND NEW.description='Groceries' BEGIN SELECT RAISE(ABORT,'injected revised transaction failure'); END;",
            )
            .unwrap();
        assert!(matches!(
            repository.finalize(&revised_batch),
            Err(ImportError::Database(_))
        ));
        {
            let c = db.connection().lock().unwrap();
            let (balance, reversals): (i64, i64) = c
                .query_row(
                    "SELECT (SELECT SUM(amount_minor) FROM account_entries),(SELECT COUNT(*) FROM financial_transactions WHERE reversal_of_transaction_id IS NOT NULL)",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!((balance, reversals), (-1250, 0));
        }
        db.connection()
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_revision_replacement")
            .unwrap();
        assert_eq!(repository.finalize(&revised_batch).unwrap(), 1);
        let c = db.connection().lock().unwrap();
        let balance: i64 = c
            .query_row("SELECT SUM(amount_minor) FROM account_entries", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(balance, -1500);
        let reversal: String = c
            .query_row(
                "SELECT id FROM financial_transactions WHERE reversal_of_transaction_id=?1",
                [&old_transaction],
                |row| row.get(0),
            )
            .unwrap();
        let audit_reason: String = c
            .query_row(
                "SELECT reason FROM transaction_audit_log WHERE transaction_id=?1 AND action='reversed'",
                [&old_transaction],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!reversal.is_empty());
        assert!(audit_reason.contains(&revised_batch));
    }

    #[test]
    fn revised_changes_require_resolution_and_keep_original_does_not_change_ledger() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Memo".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        let original_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "original.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"original statement".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &original_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,12.50\n",
                &mapping,
            )
            .unwrap();
        let original_record = repository.list_records(&original_batch).unwrap().remove(0);
        repository.review_record(&original_record.id, true).unwrap();
        repository.finalize(&original_batch).unwrap();
        let revised_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "revised.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: Some(original_batch),
                source_file_contents: b"revised statement".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &revised_batch,
                "Date,Memo,Amount\n2026-11-05,Groceries,15.00\n",
                &mapping,
            )
            .unwrap();
        let change = repository
            .compare_revision(&revised_batch)
            .unwrap()
            .remove(0);
        assert!(matches!(
            repository.finalize(&revised_batch),
            Err(ImportError::InvalidBatch)
        ));
        assert!(matches!(
            repository.set_revision_resolution(&change.id, "merge"),
            Err(ImportError::InvalidBatch)
        ));
        repository
            .set_revision_resolution(&change.id, "keep_original")
            .unwrap();
        assert_eq!(repository.finalize(&revised_batch).unwrap(), 0);
        let c = db.connection().lock().unwrap();
        let (count, balance): (i64, i64) = c
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(amount_minor),0) FROM account_entries",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((count, balance), (1, -1250));
    }

    #[test]
    fn finalization_posts_import_and_is_one_shot() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let record = staged_expense(&repo, &batch);
        repo.review_record(&record, true).unwrap();
        assert_eq!(repo.finalize(&batch).unwrap(), 1);
        assert!(matches!(
            repo.finalize(&batch),
            Err(ImportError::InvalidBatch)
        ));
        let c = db.connection().lock().unwrap();
        let posted: (i64, i64) = c
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(amount_minor),0) FROM account_entries",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(posted, (1, -1250));
        let linked: bool = c
            .query_row(
                "SELECT linked_transaction_id IS NOT NULL FROM imported_records WHERE id=?1",
                [&record],
                |r| r.get(0),
            )
            .unwrap();
        assert!(linked);
        let audit: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM transaction_audit_log WHERE action='created'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit, 1);
    }

    #[test]
    fn finalization_imports_valid_pending_rows_without_manual_review() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        staged_expense(&repo, &batch);
        assert_eq!(repo.finalize(&batch).unwrap(), 1);
        let entries: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entries, 1);
    }

    #[test]
    fn finalization_rolls_back_everything_when_ledger_insert_fails() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let record = staged_expense(&repo, &batch);
        repo.review_record(&record, true).unwrap();
        db.connection().lock().unwrap().execute_batch("CREATE TRIGGER fail_import_entry BEFORE INSERT ON account_entries BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
        assert!(repo.finalize(&batch).is_err());
        let c = db.connection().lock().unwrap();
        let status: String = c
            .query_row(
                "SELECT status FROM import_batches WHERE id=?1",
                [&batch],
                |r| r.get(0),
            )
            .unwrap();
        let (txns, entries): (i64,i64) = c.query_row("SELECT (SELECT COUNT(*) FROM financial_transactions WHERE source_type='import'), (SELECT COUNT(*) FROM account_entries)", [], |r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        let linked: bool = c
            .query_row(
                "SELECT linked_transaction_id IS NOT NULL FROM imported_records WHERE id=?1",
                [&record],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "review");
        assert_eq!((txns, entries), (0, 0));
        assert!(!linked);
    }

    #[test]
    fn statement_balance_discrepancies_do_not_block_import() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let row = staged_expense(&repo, &batch);
        repo.review_record(&row, true).unwrap();
        let reconciled = repo.reconcile(&batch, Some(10_000), Some(8_750)).unwrap();
        assert_eq!(reconciled.outcome, "reconciled");
        assert_eq!(reconciled.expected_closing_balance_minor, Some(8_750));
        repo.correct_record(&row, "2026-11-05T00:00:00Z", 1_300, "expense", None)
            .unwrap();
        let discrepancy = repo.get_reconciliation(&batch).unwrap();
        assert_eq!(discrepancy.outcome, "discrepancy");
        assert_eq!(discrepancy.difference_minor, Some(50));
        assert_eq!(repo.finalize(&batch).unwrap(), 1);
        let status: String = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT status FROM import_batches WHERE id=?1",
                [&batch],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "finalized");
    }

    #[test]
    fn reconciliation_rejects_unvalidated_rows_and_can_be_unavailable() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "d".into(),
            description_column: None,
            amount_column: "a".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(&batch, "d,a\nbad,12.00\n", &mapping)
            .unwrap();
        assert!(matches!(
            repo.reconcile(&batch, Some(10_000), Some(8_800)),
            Err(ImportError::InvalidBatch)
        ));
        let unavailable = repo.reconcile(&batch, None, None).unwrap();
        assert_eq!(unavailable.outcome, "unavailable");
        assert!(unavailable.difference_minor.is_none());
    }

    #[test]
    fn imported_split_allocations_are_validated_and_finalized_atomically() {
        let db = db();
        account(&db);
        db.connection().lock().unwrap().execute_batch("INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense'),('home','Home','expense');").unwrap();
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let row = staged_expense(&repo, &batch);
        assert!(matches!(
            repo.set_record_allocations(
                &row,
                &[
                    StagedCategoryAllocation {
                        category_id: "food".into(),
                        amount_minor: 600
                    },
                    StagedCategoryAllocation {
                        category_id: "home".into(),
                        amount_minor: 600
                    }
                ]
            ),
            Err(ImportError::InvalidBatch)
        ));
        repo.set_record_allocations(
            &row,
            &[
                StagedCategoryAllocation {
                    category_id: "food".into(),
                    amount_minor: 700,
                },
                StagedCategoryAllocation {
                    category_id: "home".into(),
                    amount_minor: 550,
                },
            ],
        )
        .unwrap();
        repo.review_record(&row, true).unwrap();
        repo.finalize(&batch).unwrap();
        let c = db.connection().lock().unwrap();
        let (count, total): (i64, i64) = c
            .query_row(
                "SELECT COUNT(*),SUM(amount_minor) FROM category_allocations",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((count, total), (2, 1250));
    }

    #[test]
    fn finalizes_confirmed_records_without_category_allocations() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "uncategorized.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let row = staged_expense(&repo, &batch);
        repo.review_record(&row, true).unwrap();

        assert_eq!(repo.finalize(&batch).unwrap(), 1);
        let connection = db.connection().lock().unwrap();
        let transaction_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |r| {
                r.get(0)
            })
            .unwrap();
        let allocation_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM category_allocations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(transaction_count, 1);
        assert_eq!(allocation_count, 0);
    }

    #[test]
    fn import_category_rules_create_categories_and_enforce_unique_terms_by_type() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let rules = repository
            .save_category_rule(
                None,
                "Food",
                "expense",
                &["Inland Hotel".into(), "Hope Traders".into()],
            )
            .unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].category_name, "Food");
        assert_eq!(rules[0].category_type, "expense");
        let category_id = rules[0].category_id.clone();

        assert!(matches!(
            repository.save_category_rule(None, "Meals", "expense", &["inland hotel".into()]),
            Err(ImportError::InvalidBatch)
        ));
        let replacement = repository
            .save_category_rule(
                Some(&category_id),
                "Food",
                "expense",
                &["Restaurant".into()],
            )
            .unwrap();
        assert_eq!(replacement.len(), 1);
        assert_eq!(replacement[0].match_text, "Restaurant");

        repository.delete_category_rule(&category_id).unwrap();
        assert!(repository.list_category_rules().unwrap().is_empty());
        let category_exists: bool = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND name='Food' AND archived_at IS NULL)",
                [&category_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(category_exists);
    }

    #[test]
    fn suggests_repeated_import_descriptions_by_direction_and_applies_accepted_categories() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let expense_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "expenses.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let expense_mapping = CsvImportMapping {
            date_column: "date".into(),
            description_column: Some("details".into()),
            amount_column: "amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repository
            .stage_csv(
                &expense_batch,
                "date,details,amount\n2026-11-01,Payment to Green Valley Hotel ref 9001,10.00\n2026-11-02,Purchase at Green Valley Hotel grocery ref 9002,11.00\n2026-11-03,Payment to Green Valley Hotel ref 9003,12.00\n2026-11-04,Payment to One Off Shop ref 8001,13.00\n2026-11-05,Payment to One Off Shop ref 8002,14.00\n",
                &expense_mapping,
            )
            .unwrap();
        repository.finalize(&expense_batch).unwrap();

        let income_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "income.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let income_mapping = CsvImportMapping {
            positive_amount_type: "income".into(),
            ..expense_mapping.clone()
        };
        repository
            .stage_csv(
                &income_batch,
                "date,details,amount\n2026-11-06,Money received from Example Consulting ref 7001,20.00\n2026-11-07,Money received from Example Consulting ref 7002,21.00\n2026-11-08,Money received from Example Consulting ref 7003,22.00\n",
                &income_mapping,
            )
            .unwrap();
        repository.finalize(&income_batch).unwrap();

        let suggestions = repository.list_category_suggestions().unwrap();
        assert!(suggestions.iter().any(|suggestion| {
            suggestion.category_type == "expense"
                && suggestion.match_text == "green valley hotel"
                && suggestion.category_name == "Green Valley Hotel"
                && suggestion.occurrence_count == 3
        }));
        assert!(suggestions.iter().any(|suggestion| {
            suggestion.category_type == "income"
                && suggestion.match_text == "example consulting"
                && suggestion.occurrence_count == 3
        }));
        assert!(
            !suggestions
                .iter()
                .any(|suggestion| { suggestion.match_text == "one off shop" })
        );

        let applied = repository
            .apply_category_suggestion("expense", "green valley hotel")
            .unwrap();
        assert_eq!(applied.category_name, "Green Valley Hotel");
        assert_eq!(applied.categorized_transaction_count, 3);
        let category_id: String = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT id FROM categories WHERE name='Green Valley Hotel' AND category_type='expense'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let categorized_count: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM category_allocations WHERE category_id=?1",
                [&category_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(categorized_count, 3);
        assert!(
            !repository
                .list_category_suggestions()
                .unwrap()
                .iter()
                .any(|suggestion| suggestion.category_type == "expense")
        );

        let future_batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "future.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &future_batch,
                "date,details,amount\n2026-11-09,Payment to Green-Valley Hotel,25.00\n",
                &expense_mapping,
            )
            .unwrap();
        repository.finalize(&future_batch).unwrap();
        let categorized_count: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM category_allocations WHERE category_id=?1",
                [&category_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(categorized_count, 4);
    }

    #[test]
    fn imports_safe_rows_with_description_categories_and_excludes_unsafe_rows() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let food = repository
            .save_category_rule(
                None,
                "Food",
                "expense",
                &["inland hotel".into(), "hope traders".into()],
            )
            .unwrap()[0]
            .category_id
            .clone();
        repository
            .save_category_rule(None, "Hotel", "expense", &["hotel".into()])
            .unwrap();
        let batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "date".into(),
            description_column: Some("memo".into()),
            amount_column: "amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repository
            .stage_csv(
                &batch,
                "date,memo,amount\n2026-11-05,Inland Hotel,25.00\n2026-11-05,Shop,12.00\n2026-11-05,Inland Hotel,25.00\nbad,Other,5.00\n",
                &mapping,
            )
            .unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE import_batches SET reconciliation_status='discrepancy' WHERE id=?1",
                [&batch],
            )
            .unwrap();

        assert_eq!(repository.finalize(&batch).unwrap(), 2);
        let records = repository.list_records(&batch).unwrap();
        assert_eq!(records[0].review_status, "confirmed");
        assert_eq!(records[1].review_status, "confirmed");
        assert_eq!(records[2].review_status, "skipped");
        assert_eq!(records[3].review_status, "skipped");
        let connection = db.connection().lock().unwrap();
        let (transaction_count, categorized): (i64, i64) = connection
            .query_row(
                "SELECT (SELECT COUNT(*) FROM financial_transactions),(SELECT COUNT(*) FROM category_allocations WHERE category_id=?1)",
                [&food],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((transaction_count, categorized), (2, 1));
        let matched_category: String = connection
            .query_row(
                "SELECT ca.category_id FROM category_allocations ca JOIN financial_transactions t ON t.id=ca.transaction_id WHERE t.description='Inland Hotel'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(matched_category, food);
    }

    #[test]
    fn infers_direction_from_incoming_and_outgoing_columns() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "direction.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "date".into(),
            description_column: Some("memo".into()),
            amount_column: "Paid In".into(),
            positive_amount_type: "income".into(),
            date_format: "iso_date".into(),
        };
        repository
            .stage_csv_with_directions(
                &batch,
                "date,memo,Paid In,Withdrawn\n2026-11-05,Received,25.00,0.00\n2026-11-06,Purchase,0.00,12.50\n2026-11-07,Ambiguous,2.00,1.00\n",
                &mapping,
                Some("Paid In"),
                Some("Withdrawn"),
            )
            .unwrap();
        let records = repository.list_records(&batch).unwrap();
        assert_eq!(records[0].transaction_type.as_deref(), Some("income"));
        assert_eq!(records[0].validation_status, "valid");
        assert_eq!(records[1].transaction_type.as_deref(), Some("expense"));
        assert_eq!(records[1].validation_status, "valid");
        assert_eq!(records[2].transaction_type, None);
        assert_eq!(records[2].validation_status, "uncertain");
    }

    #[test]
    fn leaves_unsigned_generic_amounts_unclassified() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "unsigned.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        repository
            .stage_csv(
                &batch,
                "date,memo,amount\n2026-11-05,Shop,12.50\n",
                &CsvImportMapping {
                    date_column: "date".into(),
                    description_column: Some("memo".into()),
                    amount_column: "amount".into(),
                    positive_amount_type: "unknown".into(),
                    date_format: "iso_date".into(),
                },
            )
            .unwrap();
        let record = repository.list_records(&batch).unwrap().remove(0);
        assert_eq!(record.normalized_amount_minor, Some(1250));
        assert_eq!(record.transaction_type, None);
        assert_eq!(record.validation_status, "uncertain");
    }

    #[test]
    fn same_date_amount_and_type_with_different_descriptions_are_not_duplicates() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();

        repository
            .stage_csv(
                &batch,
                "date,memo,amount\n2026-11-05,Shop A,12.50\n2026-11-05,Shop B,12.50\n",
                &CsvImportMapping {
                    date_column: "date".into(),
                    description_column: Some("memo".into()),
                    amount_column: "amount".into(),
                    positive_amount_type: "expense".into(),
                    date_format: "iso_date".into(),
                },
            )
            .unwrap();

        let staged = repository.list_records(&batch).unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE imported_records SET duplicate_status='probable' WHERE id=?1",
                [&staged[1].id],
            )
            .unwrap();
        let records = repository.list_records(&batch).unwrap();
        assert_eq!(records.len(), 2);
        assert!(
            records
                .iter()
                .all(|record| record.duplicate_status == "unmatched")
        );
    }

    #[test]
    fn bulk_confirmation_leaves_uncertain_and_possible_duplicates_for_manual_review() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "date".into(),
            description_column: Some("memo".into()),
            amount_column: "amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(
            &batch,
            "date,memo,amount\n2026-11-05,Shop,12.50\n2026-11-05,Shop,12.50\nbad,Other,5.00\n",
            &mapping,
        )
        .unwrap();

        assert_eq!(repo.confirm_valid_records(&batch).unwrap(), 1);
        let records = repo.list_records(&batch).unwrap();
        assert_eq!(records[0].review_status, "confirmed");
        assert_eq!(records[1].review_status, "pending");
        assert_eq!(records[1].duplicate_status, "probable");
        assert_eq!(records[2].validation_status, "uncertain");
        assert_eq!(records[2].review_status, "pending");
        let transaction_count: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(transaction_count, 0);

        let parent = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "original.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: None,
                source_file_contents: b"original statement".to_vec(),
            })
            .unwrap();
        repo.stage_csv(
            &parent,
            "date,memo,amount\n2026-11-07,Market,5.00\n",
            &mapping,
        )
        .unwrap();
        let parent_record = repo.list_records(&parent).unwrap().remove(0);
        repo.review_record(&parent_record.id, true).unwrap();
        repo.finalize(&parent).unwrap();

        let revision = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "revised.csv".into(),
                source_format: "csv".into(),
                revision_of_batch_id: Some(parent),
                source_file_contents: b"revised statement".to_vec(),
            })
            .unwrap();
        repo.stage_csv(
            &revision,
            "date,memo,amount\n2026-11-06,Market,5.00\n",
            &mapping,
        )
        .unwrap();
        assert!(matches!(
            repo.confirm_valid_records(&revision),
            Err(ImportError::InvalidBatch)
        ));
        assert_eq!(
            repo.list_records(&revision).unwrap()[0].review_status,
            "pending"
        );
    }

    #[test]
    fn stages_pdfjs_extracted_page_text_as_uncertain_review_rows() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.pdf".into(),
                source_format: "pdf".into(),
                revision_of_batch_id: None,
                source_file_contents: b"retained PDF source".to_vec(),
            })
            .unwrap();
        assert_eq!(
            repository
                .stage_pdf_pages(&batch, vec!["2026-11-06 Market purchase 12.50".into()])
                .unwrap(),
            1
        );
        let records = repository.list_records(&batch).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].source_row_reference.contains("Page 1, line 1"));
        assert_eq!(records[0].validation_status, "uncertain");
        assert_eq!(
            records[0].normalized_date.as_deref(),
            Some("2026-11-06T00:00:00Z")
        );
        assert_eq!(records[0].normalized_amount_minor, Some(1250));
        assert_eq!(records[0].transaction_type, None);
        assert!(
            records[0]
                .original_description
                .as_deref()
                .unwrap_or_default()
                .contains("Market purchase")
        );
        assert!(matches!(
            repository.stage_pdf_pages(&batch, vec![String::new()]),
            Err(ImportError::PdfNoExtractableText)
        ));
        assert_eq!(
            repository.source_file_contents(&batch).unwrap(),
            b"retained PDF source"
        );
    }

    #[test]
    fn stages_table_extracted_pdf_rows_with_transaction_amounts_for_review() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let batch = repository
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.pdf".into(),
                source_format: "pdf".into(),
                revision_of_batch_id: None,
                source_file_contents: b"retained PDF source".to_vec(),
            })
            .unwrap();

        assert_eq!(
            repository
                .stage_pdf_pages(
                    &batch,
                    vec![
                        "2026-10-31 14:43:07 Airtime Purchase -15.00\n2026-10-30 13:45:28 Cash received 20.00".into(),
                    ],
                )
                .unwrap(),
            2
        );

        let records = repository.list_records(&batch).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].normalized_date.as_deref(),
            Some("2026-10-31T00:00:00Z")
        );
        assert_eq!(records[0].normalized_amount_minor, Some(1500));
        assert_eq!(records[0].transaction_type, None);
        assert_eq!(records[0].validation_status, "uncertain");
        assert!(
            records[0]
                .original_description
                .as_deref()
                .unwrap_or_default()
                .contains("Airtime Purchase")
        );
        assert_eq!(
            records[1].normalized_date.as_deref(),
            Some("2026-10-30T00:00:00Z")
        );
        assert_eq!(records[1].normalized_amount_minor, Some(2000));
        assert!(
            records[1]
                .original_description
                .as_deref()
                .unwrap_or_default()
                .contains("Cash received")
        );
    }

    #[test]
    fn stages_detected_pdf_table_direction_and_keeps_ambiguous_rows_unconfirmed() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "statement.pdf".into(),
                source_format: "pdf".into(),
                revision_of_batch_id: None,
                source_file_contents: b"retained PDF source".to_vec(),
            })
            .unwrap();

        assert_eq!(
            repo.stage_pdf_table_pages(
                &batch,
                vec!["2026-10-31 Airtime -15.00\n2026-10-31 Airtime -15.00\n2026-10-31 Bus -15.00\n2026-10-30 Cash received 20.00\n2026-10-29 Failed 5.00\n2026-10-27 Bus 15".into()],
                vec![
                    Some("expense".into()),
                    Some("expense".into()),
                    Some("expense".into()),
                    Some("income".into()),
                    None,
                    Some("expense".into()),
                ],
            )
            .unwrap(),
            6
        );

        let records = repo.list_records(&batch).unwrap();
        assert_eq!(records[0].validation_status, "valid");
        assert_eq!(records[0].transaction_type.as_deref(), Some("expense"));
        assert_eq!(records[0].review_status, "pending");
        assert_eq!(records[1].validation_status, "valid");
        assert_eq!(records[1].transaction_type.as_deref(), Some("expense"));
        assert_eq!(records[1].duplicate_status, "probable");
        assert_eq!(records[2].validation_status, "valid");
        assert_eq!(records[2].transaction_type.as_deref(), Some("expense"));
        assert_eq!(records[2].duplicate_status, "unmatched");
        assert_eq!(records[3].validation_status, "valid");
        assert_eq!(records[3].transaction_type.as_deref(), Some("income"));
        assert_eq!(records[4].validation_status, "uncertain");
        assert_eq!(records[4].transaction_type, None);
        assert_eq!(records[5].validation_status, "valid");
        assert_eq!(records[5].normalized_amount_minor, Some(1500));
        assert_eq!(records[5].transaction_type.as_deref(), Some("expense"));
        let connection = db.connection().lock().unwrap();
        let entries: i64 = connection
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entries, 0);
    }

    #[test]
    fn parses_mapped_xlsx_rows_and_preserves_sheet_row_reference() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "sample.xlsx".into(),
                source_format: "xlsx".into(),
                revision_of_batch_id: None,
                source_file_contents: b"statement fixture".to_vec(),
            })
            .unwrap();
        let bytes = workbook_fixture();
        assert_eq!(
            repo.xlsx_sheet_names(bytes.clone()).unwrap(),
            vec!["Transactions"]
        );
        assert_eq!(
            repo.xlsx_headers(bytes.clone(), "Transactions", 1).unwrap(),
            vec!["Date", "Amount", "Memo"]
        );
        assert_eq!(
            repo.xlsx_rows(bytes.clone(), "Transactions", 1).unwrap(),
            vec![
                vec!["Date", "Amount", "Memo"],
                vec!["2026-11-05", "12.5", "Market"],
                vec!["2026-11-06", "12.5", "Calculated total"],
            ]
        );
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Memo".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        assert_eq!(
            repo.stage_xlsx(&batch, bytes, "Transactions", 1, &mapping)
                .unwrap_or_else(|error| panic!(
                    "xlsx staging failed: {error:?}; staged={:?}",
                    repo.list_records(&batch)
                )),
            2
        );
        let records = repo.list_records(&batch).unwrap();
        let record = &records[0];
        assert_eq!(record.source_row_reference, "Transactions!2");
        assert_eq!(record.normalized_amount_minor, Some(1250));
        assert_eq!(record.original_description.as_deref(), Some("Market"));
        assert_eq!(records[1].source_row_reference, "Transactions!3");
        assert_eq!(records[1].validation_status, "uncertain");
    }
}

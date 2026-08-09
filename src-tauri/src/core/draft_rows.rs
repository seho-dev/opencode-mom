#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryMappingDraftRow {
    pub id: String,
    pub category_name: String,
    pub model_ref: String,
    pub is_known_key: bool,
}

impl CategoryMappingDraftRow {
    pub fn known(id: &str, category_name: &str, model_ref: &str) -> Self {
        Self {
            id: id.to_owned(),
            category_name: category_name.to_owned(),
            model_ref: model_ref.to_owned(),
            is_known_key: true,
        }
    }

    pub fn custom(id: &str, category_name: &str, model_ref: &str) -> Self {
        Self {
            id: id.to_owned(),
            category_name: category_name.to_owned(),
            model_ref: model_ref.to_owned(),
            is_known_key: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMappingDraftRow {
    pub id: String,
    pub agent_name: String,
    pub model_ref: String,
    pub is_known_key: bool,
}

impl AgentMappingDraftRow {
    pub fn known(id: &str, agent_name: &str, model_ref: &str) -> Self {
        Self {
            id: id.to_owned(),
            agent_name: agent_name.to_owned(),
            model_ref: model_ref.to_owned(),
            is_known_key: true,
        }
    }

    pub fn custom(id: &str, agent_name: &str, model_ref: &str) -> Self {
        Self {
            id: id.to_owned(),
            agent_name: agent_name.to_owned(),
            model_ref: model_ref.to_owned(),
            is_known_key: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRowWarning {
    pub row_index: usize,
    pub message: String,
}

pub fn category_duplicate_warnings(rows: &[CategoryMappingDraftRow]) -> Vec<DraftRowWarning> {
    duplicate_warnings(
        rows,
        |row| row.category_name.as_str(),
        |row| row.is_known_key,
        "category",
    )
}

pub fn agent_duplicate_warnings(rows: &[AgentMappingDraftRow]) -> Vec<DraftRowWarning> {
    duplicate_warnings(
        rows,
        |row| row.agent_name.as_str(),
        |row| row.is_known_key,
        "agent",
    )
}

fn duplicate_warnings<Row>(
    rows: &[Row],
    name: fn(&Row) -> &str,
    is_known_key: fn(&Row) -> bool,
    label: &str,
) -> Vec<DraftRowWarning> {
    rows.iter()
        .enumerate()
        .filter_map(|(row_index, row)| {
            let trimmed_name = name(row).trim();
            if is_known_key(row) || trimmed_name.is_empty() {
                return None;
            }

            let duplicate = rows.iter().enumerate().any(|(other_index, other)| {
                row_index != other_index
                    && !name(other).trim().is_empty()
                    && name(other).trim().eq_ignore_ascii_case(trimmed_name)
            });
            duplicate.then(|| DraftRowWarning {
                row_index,
                message: format!("Duplicate {label} name \"{trimmed_name}\"."),
            })
        })
        .collect()
}

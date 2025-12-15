//! Matrix expansion for parallel job generation.

use oxide_core::ids::{JobId, MatrixId};
use oxide_core::pipeline::StageDefinition;
use std::collections::HashMap;

/// A single job in an expanded matrix.
#[derive(Debug, Clone)]
pub struct MatrixJob {
    pub id: JobId,
    pub matrix_id: MatrixId,
    pub stage_name: String,
    pub index: usize,
    pub variables: HashMap<String, serde_json::Value>,
    pub display_name: String,
}

/// Result of matrix expansion.
#[derive(Debug, Clone)]
pub struct MatrixExpansion {
    pub matrix_id: MatrixId,
    pub stage_name: String,
    pub jobs: Vec<MatrixJob>,
    pub fail_fast: bool,
    pub max_parallel: Option<u32>,
}

/// Expander for matrix configurations.
pub struct MatrixExpander;

impl MatrixExpander {
    pub fn new() -> Self {
        Self
    }

    /// Expand a stage's matrix configuration into individual jobs.
    pub fn expand(&self, stage: &StageDefinition) -> Option<MatrixExpansion> {
        let matrix = stage.matrix.as_ref()?;
        let matrix_id = MatrixId::default();

        let mut combinations = self.generate_combinations(&matrix.dimensions);

        // Apply includes
        for include in &matrix.include {
            if !combinations.contains(include) {
                combinations.push(include.clone());
            }
        }

        // Apply excludes
        combinations.retain(|combo| {
            !matrix
                .exclude
                .iter()
                .any(|exclude| self.matches_exclude(combo, exclude))
        });

        let jobs: Vec<MatrixJob> = combinations
            .into_iter()
            .enumerate()
            .map(|(idx, vars)| {
                let display_name = self.format_display_name(&stage.name, &vars);
                MatrixJob {
                    id: JobId::default(),
                    matrix_id,
                    stage_name: stage.name.clone(),
                    index: idx,
                    variables: vars,
                    display_name,
                }
            })
            .collect();

        Some(MatrixExpansion {
            matrix_id,
            stage_name: stage.name.clone(),
            jobs,
            fail_fast: matrix.fail_fast,
            max_parallel: matrix.max_parallel,
        })
    }

    fn generate_combinations(
        &self,
        dimensions: &HashMap<String, serde_json::Value>,
    ) -> Vec<HashMap<String, serde_json::Value>> {
        // Filter to only array dimensions (exclude include/exclude/fail_fast/max_parallel)
        let array_dims: Vec<(&String, &Vec<serde_json::Value>)> = dimensions
            .iter()
            .filter_map(|(k, v)| v.as_array().map(|arr| (k, arr)))
            .collect();

        if array_dims.is_empty() {
            return vec![HashMap::new()];
        }

        let mut result = vec![HashMap::new()];

        for (key, values) in array_dims {
            let mut new_result = Vec::new();

            for combo in result {
                for value in values {
                    let mut new_combo = combo.clone();
                    new_combo.insert(key.clone(), value.clone());
                    new_result.push(new_combo);
                }
            }

            result = new_result;
        }

        result
    }

    fn matches_exclude(
        &self,
        combo: &HashMap<String, serde_json::Value>,
        exclude: &HashMap<String, serde_json::Value>,
    ) -> bool {
        exclude
            .iter()
            .all(|(key, value)| combo.get(key) == Some(value))
    }

    fn format_display_name(
        &self,
        stage_name: &str,
        vars: &HashMap<String, serde_json::Value>,
    ) -> String {
        if vars.is_empty() {
            return stage_name.to_string();
        }

        let parts: Vec<String> = vars
            .iter()
            .map(|(k, v)| {
                let v_str = match v {
                    serde_json::Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                format!("{}={}", k, v_str)
            })
            .collect();

        format!("{} ({})", stage_name, parts.join(", "))
    }
}

impl Default for MatrixExpander {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxide_core::pipeline::{MatrixConfig, StepDefinition};

    #[test]
    fn test_matrix_expansion() {
        let mut dimensions = HashMap::new();
        dimensions.insert(
            "os".to_string(),
            serde_json::json!(["linux", "macos"]),
        );
        dimensions.insert(
            "version".to_string(),
            serde_json::json!(["18", "20", "22"]),
        );

        let stage = StageDefinition {
            name: "test".to_string(),
            display_name: None,
            depends_on: vec![],
            condition: None,
            environment: None,
            variables: Default::default(),
            steps: vec![StepDefinition {
                name: "run".to_string(),
                display_name: None,
                plugin: None,
                run: Some("npm test".to_string()),
                shell: "bash".to_string(),
                working_directory: None,
                environment: None,
                variables: Default::default(),
                secrets: vec![],
                condition: None,
                timeout_minutes: 30,
                retry: None,
                continue_on_error: false,
                outputs: vec![],
            }],
            parallel: false,
            timeout_minutes: None,
            retry: None,
            agent: None,
            matrix: Some(MatrixConfig {
                dimensions,
                include: vec![],
                exclude: vec![],
                fail_fast: true,
                max_parallel: Some(4),
            }),
        };

        let expander = MatrixExpander::new();
        let expansion = expander.expand(&stage).unwrap();

        assert_eq!(expansion.jobs.len(), 6); // 2 OS × 3 versions
        assert!(expansion.fail_fast);
        assert_eq!(expansion.max_parallel, Some(4));
    }

    #[test]
    fn test_matrix_with_exclude() {
        let mut dimensions = HashMap::new();
        dimensions.insert(
            "os".to_string(),
            serde_json::json!(["linux", "macos"]),
        );
        dimensions.insert(
            "arch".to_string(),
            serde_json::json!(["amd64", "arm64"]),
        );

        let mut exclude = HashMap::new();
        exclude.insert(
            "os".to_string(),
            serde_json::Value::String("macos".to_string()),
        );
        exclude.insert(
            "arch".to_string(),
            serde_json::Value::String("amd64".to_string()),
        );

        let stage = StageDefinition {
            name: "build".to_string(),
            display_name: None,
            depends_on: vec![],
            condition: None,
            environment: None,
            variables: Default::default(),
            steps: vec![],
            parallel: false,
            timeout_minutes: None,
            retry: None,
            agent: None,
            matrix: Some(MatrixConfig {
                dimensions,
                include: vec![],
                exclude: vec![exclude],
                fail_fast: true,
                max_parallel: None,
            }),
        };

        let expander = MatrixExpander::new();
        let expansion = expander.expand(&stage).unwrap();

        // 2x2 = 4, minus 1 excluded = 3
        assert_eq!(expansion.jobs.len(), 3);
    }
}

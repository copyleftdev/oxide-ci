use std::collections::HashMap;
use regex::Regex;

/// Context for variable interpolation.
#[derive(Debug, Clone, Default)]
pub struct InterpolationContext {
    /// Pipeline and stage variables
    pub variables: HashMap<String, String>,
    /// Step outputs: "step_name.output_key" -> value
    pub outputs: HashMap<String, String>,
    /// Matrix values for current job
    pub matrix: HashMap<String, String>,
    /// Secrets to mask in output
    pub secrets: HashMap<String, String>,
}

impl InterpolationContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolate variables in a string.
    ///
    /// Supports:
    /// - `${{ variable }}` - direct variable lookup
    /// - `${{ env.VAR }}` - environment variable
    /// - `${{ matrix.key }}` - matrix value
    /// - `${{ steps.name.outputs.key }}` - step output
    pub fn interpolate(&self, input: &str) -> String {
        // Simple regex for ${{ ... }}
        // Note: nesting not supported in this simple version
        let re = Regex::new(r"\$\{\{\s*([^}]+)\s*\}\}").unwrap();

        re.replace_all(input, |caps: &regex::Captures| {
            let expr = caps.get(1).map_or("", |m| m.as_str()).trim();
            self.resolve_expression(expr)
        })
        .to_string()
    }

    /// Resolve a single expression.
    fn resolve_expression(&self, expr: &str) -> String {
        // Handle env.VAR
        if let Some(var_name) = expr.strip_prefix("env.") {
            return self
                .variables
                .get(var_name)
                .cloned()
                .or_else(|| std::env::var(var_name).ok())
                .unwrap_or_default();
        }

        // Handle matrix.key
        if let Some(key) = expr.strip_prefix("matrix.") {
            return self.matrix.get(key).cloned().unwrap_or_default();
        }

        // Handle steps.name.outputs.key
        if let Some(rest) = expr.strip_prefix("steps.") {
            // Finding .outputs.
            if let Some(outputs_idx) = rest.find(".outputs.") {
                let step_name = &rest[..outputs_idx];
                let output_key = &rest[outputs_idx + 9..]; // ".outputs." is 9 chars
                let lookup_key = format!("{}.{}", step_name, output_key);
                return self.outputs.get(&lookup_key).cloned().unwrap_or_default();
            }
        }

        // Direct variable lookup
        self.variables.get(expr).cloned().unwrap_or_default()
    }

    /// Mask secrets in the input string.
    pub fn mask_secrets(&self, input: &str) -> String {
        let mut output = input.to_string();
        for value in self.secrets.values() {
            if !value.is_empty() {
                output = output.replace(value, "***");
            }
        }
        output
    }

    /// Evaluate a condition expression.
    pub fn evaluate_condition(&self, condition: &crate::pipeline::ConditionExpression) -> bool {
        match condition {
            crate::pipeline::ConditionExpression::Simple(expr) => self.evaluate_string_expression(expr),
            crate::pipeline::ConditionExpression::Structured { if_expr, unless } => {
                if let Some(expr) = if_expr
                    && !self.evaluate_string_expression(expr)
                {
                    return false;
                }
                if let Some(expr) = unless
                    && self.evaluate_string_expression(expr)
                {
                    return false;
                }
                true
            }
        }
    }

    /// Evaluate a simple string expression (equality, inequality, contains).
    fn evaluate_string_expression(&self, expr: &str) -> bool {
        let interpolated = self.interpolate(expr);
        let trimmed = interpolated.trim();

        // Boolean literals
        if trimmed == "true" {
            return true;
        }
        if trimmed == "false" {
            return false;
        }

        // Operators
        if let Some((left, right)) = trimmed.split_once("==") {
            return left.trim() == right.trim();
        }
        if let Some((left, right)) = trimmed.split_once("!=") {
            return left.trim() != right.trim();
        }
        if let Some((left, right)) = trimmed.split_once(" contains ") {
            return left.trim().contains(right.trim());
        }

        // Default to false if not recognized (safe default)
        false
    }
}

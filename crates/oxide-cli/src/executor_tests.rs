//! Tests for the executor module.

#[cfg(test)]
mod tests {
    use crate::executor::ExecutionContext;
    use std::path::PathBuf;

    #[test]
    fn test_interpolate_simple_variable() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        ctx.variables.insert("NAME".to_string(), "world".to_string());

        assert_eq!(ctx.interpolate("Hello ${{ NAME }}!"), "Hello world!");
    }

    #[test]
    fn test_interpolate_env_variable() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        ctx.variables.insert("MY_VAR".to_string(), "test_value".to_string());

        assert_eq!(ctx.interpolate("Value: ${{ env.MY_VAR }}"), "Value: test_value");
    }

    #[test]
    fn test_interpolate_matrix_variable() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        ctx.matrix.insert("os".to_string(), "linux".to_string());
        ctx.matrix.insert("arch".to_string(), "amd64".to_string());

        assert_eq!(
            ctx.interpolate("Building for ${{ matrix.os }}-${{ matrix.arch }}"),
            "Building for linux-amd64"
        );
    }

    #[test]
    fn test_interpolate_step_outputs() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        ctx.set_output("build", "version", "1.2.3".to_string());
        ctx.set_output("build", "artifact", "app.tar.gz".to_string());

        assert_eq!(
            ctx.interpolate("Version: ${{ steps.build.outputs.version }}"),
            "Version: 1.2.3"
        );
        assert_eq!(
            ctx.interpolate("Artifact: ${{ steps.build.outputs.artifact }}"),
            "Artifact: app.tar.gz"
        );
    }

    #[test]
    fn test_interpolate_missing_variable_returns_empty() {
        let ctx = ExecutionContext::new(PathBuf::from("/tmp"));

        assert_eq!(ctx.interpolate("Value: ${{ MISSING }}"), "Value: ");
        assert_eq!(ctx.interpolate("${{ env.MISSING }}"), "");
        assert_eq!(ctx.interpolate("${{ matrix.missing }}"), "");
        assert_eq!(ctx.interpolate("${{ steps.missing.outputs.key }}"), "");
    }

    #[test]
    fn test_interpolate_multiple_variables() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        ctx.variables.insert("FIRST".to_string(), "Hello".to_string());
        ctx.variables.insert("SECOND".to_string(), "World".to_string());

        assert_eq!(
            ctx.interpolate("${{ FIRST }} ${{ SECOND }}!"),
            "Hello World!"
        );
    }

    #[test]
    fn test_interpolate_with_whitespace_variations() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        ctx.variables.insert("VAR".to_string(), "value".to_string());

        // Various whitespace patterns should all work
        assert_eq!(ctx.interpolate("${{VAR}}"), "value");
        assert_eq!(ctx.interpolate("${{ VAR }}"), "value");
        assert_eq!(ctx.interpolate("${{  VAR  }}"), "value");
        assert_eq!(ctx.interpolate("${{   VAR   }}"), "value");
    }

    #[test]
    fn test_interpolate_no_variables() {
        let ctx = ExecutionContext::new(PathBuf::from("/tmp"));

        assert_eq!(
            ctx.interpolate("No variables here"),
            "No variables here"
        );
    }

    #[test]
    fn test_parse_outputs_simple() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        let content = "version=1.0.0\nstatus=success";

        ctx.parse_outputs("build", content);

        assert_eq!(
            ctx.outputs.get("build.version"),
            Some(&"1.0.0".to_string())
        );
        assert_eq!(
            ctx.outputs.get("build.status"),
            Some(&"success".to_string())
        );
    }

    #[test]
    fn test_parse_outputs_with_empty_lines() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        let content = "key1=value1\n\nkey2=value2\n";

        ctx.parse_outputs("step", content);

        assert_eq!(ctx.outputs.get("step.key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.outputs.get("step.key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_parse_outputs_with_equals_in_value() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        let content = "equation=x=y+z";

        ctx.parse_outputs("math", content);

        assert_eq!(
            ctx.outputs.get("math.equation"),
            Some(&"x=y+z".to_string())
        );
    }

    #[test]
    fn test_set_output_and_retrieve() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        
        ctx.set_output("step1", "result", "success".to_string());
        ctx.set_output("step1", "count", "42".to_string());
        ctx.set_output("step2", "result", "failure".to_string());

        // Retrieve via interpolation
        assert_eq!(
            ctx.interpolate("${{ steps.step1.outputs.result }}"),
            "success"
        );
        assert_eq!(
            ctx.interpolate("${{ steps.step1.outputs.count }}"),
            "42"
        );
        assert_eq!(
            ctx.interpolate("${{ steps.step2.outputs.result }}"),
            "failure"
        );
    }

    #[test]
    fn test_interpolate_complex_script() {
        let mut ctx = ExecutionContext::new(PathBuf::from("/tmp"));
        ctx.variables.insert("PROJECT".to_string(), "myapp".to_string());
        ctx.variables.insert("VERSION".to_string(), "2.0.0".to_string());
        ctx.matrix.insert("target".to_string(), "x86_64-linux".to_string());
        ctx.set_output("build", "binary", "myapp-linux".to_string());

        let script = r#"
            echo "Building ${{ PROJECT }} v${{ VERSION }}"
            echo "Target: ${{ matrix.target }}"
            echo "Binary: ${{ steps.build.outputs.binary }}"
        "#;

        let result = ctx.interpolate(script);
        
        assert!(result.contains("Building myapp v2.0.0"));
        assert!(result.contains("Target: x86_64-linux"));
        assert!(result.contains("Binary: myapp-linux"));
    }
}

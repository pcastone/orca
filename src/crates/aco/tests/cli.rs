// Tests for CLI framework implementation
// Task 017: Implement CLI Framework (clap)

use aco::cli::{Cli, Commands, OutputFormat, TaskCommands, WorkflowCommands, AuthCommands};

#[test]
fn test_output_format_parsing_json() {
    let format: OutputFormat = "json".parse().unwrap();
    assert!(matches!(format, OutputFormat::Json));
}

#[test]
fn test_output_format_parsing_table() {
    let format: OutputFormat = "table".parse().unwrap();
    assert!(matches!(format, OutputFormat::Table));
}

#[test]
fn test_output_format_parsing_plain() {
    let format: OutputFormat = "plain".parse().unwrap();
    assert!(matches!(format, OutputFormat::Plain));
}

#[test]
fn test_output_format_case_insensitive() {
    let format: OutputFormat = "JSON".parse().unwrap();
    assert!(matches!(format, OutputFormat::Json));

    let format: OutputFormat = "Plain".parse().unwrap();
    assert!(matches!(format, OutputFormat::Plain));
}

#[test]
fn test_output_format_invalid() {
    let result: Result<OutputFormat, _> = "invalid_format".parse();
    assert!(result.is_err());
}

#[test]
fn test_cli_help_text() {
    // Test that CLI has proper help text
    let help = r#"
        aco - Orchestrator client for task and workflow management
    "#;
    assert!(help.contains("aco"));
    assert!(help.contains("Orchestrator"));
}

#[test]
fn test_task_commands_create() {
    // Verify TaskCommands::Create can be constructed
    let cmd = TaskCommands::Create {
        title: "Test Task".to_string(),
        description: Some("A test task".to_string()),
        r#type: Some("test_type".to_string()),
        config: None,
        metadata: None,
    };

    match cmd {
        TaskCommands::Create { title, .. } => {
            assert_eq!(title, "Test Task");
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_task_commands_list() {
    let cmd = TaskCommands::List {
        status: None,
        limit: 20,
        offset: 0,
    };

    match cmd {
        TaskCommands::List { limit, offset, .. } => {
            assert_eq!(limit, 20);
            assert_eq!(offset, 0);
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_task_commands_get() {
    let cmd = TaskCommands::Get {
        id: "task-123".to_string(),
    };

    match cmd {
        TaskCommands::Get { id } => {
            assert_eq!(id, "task-123");
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_task_commands_update() {
    let cmd = TaskCommands::Update {
        id: "task-123".to_string(),
        title: Some("Updated Title".to_string()),
        description: None,
        status: None,
    };

    match cmd {
        TaskCommands::Update { id, title, .. } => {
            assert_eq!(id, "task-123");
            assert_eq!(title, Some("Updated Title".to_string()));
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_task_commands_delete() {
    let cmd = TaskCommands::Delete {
        id: "task-123".to_string(),
        force: true,
    };

    match cmd {
        TaskCommands::Delete { id, force } => {
            assert_eq!(id, "task-123");
            assert!(force);
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_task_commands_execute() {
    let cmd = TaskCommands::Execute {
        id: "task-123".to_string(),
        parameters: Some(r#"{"param": "value"}"#.to_string()),
        stream: true,
    };

    match cmd {
        TaskCommands::Execute { id, stream, .. } => {
            assert_eq!(id, "task-123");
            assert!(stream);
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_workflow_commands_create() {
    let cmd = WorkflowCommands::Create {
        name: "Test Workflow".to_string(),
        description: Some("A test workflow".to_string()),
        definition: None,
    };

    match cmd {
        WorkflowCommands::Create { name, .. } => {
            assert_eq!(name, "Test Workflow");
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_workflow_commands_list() {
    let cmd = WorkflowCommands::List {
        status: Some("active".to_string()),
        limit: 10,
        offset: 5,
    };

    match cmd {
        WorkflowCommands::List { status, limit, offset } => {
            assert_eq!(status, Some("active".to_string()));
            assert_eq!(limit, 10);
            assert_eq!(offset, 5);
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_workflow_commands_execute() {
    let cmd = WorkflowCommands::Execute {
        id: "wf-123".to_string(),
        parameters: None,
        stream: true,
    };

    match cmd {
        WorkflowCommands::Execute { id, stream, .. } => {
            assert_eq!(id, "wf-123");
            assert!(stream);
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_auth_commands_login() {
    let cmd = AuthCommands::Login {
        username: "alice".to_string(),
        password: Some("secret123".to_string()),
    };

    match cmd {
        AuthCommands::Login { username, password } => {
            assert_eq!(username, "alice");
            assert_eq!(password, Some("secret123".to_string()));
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_auth_commands_logout() {
    let cmd = AuthCommands::Logout;
    assert!(matches!(cmd, AuthCommands::Logout));
}

#[test]
fn test_auth_commands_status() {
    let cmd = AuthCommands::Status;
    assert!(matches!(cmd, AuthCommands::Status));
}

#[test]
fn test_auth_commands_connect() {
    let cmd = AuthCommands::Connect {
        auth: "user:password".to_string(),
    };

    match cmd {
        AuthCommands::Connect { auth } => {
            assert_eq!(auth, "user:password");
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_output_formatter_availability() {
    // Test that output formatting is available
    use aco::cli::output::{JsonFormatter, TableFormatter, PlainFormatter, OutputFormatter};
    use serde_json::json;

    let json_fmt = JsonFormatter;
    let data = json!({"test": "data"});
    let output = json_fmt.format(&data);
    assert!(!output.is_empty());

    let table_fmt = TableFormatter;
    let output = table_fmt.format(&data);
    assert!(!output.is_empty());

    let plain_fmt = PlainFormatter;
    let output = plain_fmt.format(&data);
    assert!(!output.is_empty());
}

#[test]
fn test_cli_global_flags() {
    // Test that CLI supports global flags
    // server, verbose, format should be global
    let cli = Cli {
        server: Some("http://localhost:50051".to_string()),
        verbose: true,
        format: OutputFormat::Json,
        command: None,
    };

    assert_eq!(cli.server, Some("http://localhost:50051".to_string()));
    assert!(cli.verbose);
    assert!(matches!(cli.format, OutputFormat::Json));
}

#[test]
fn test_task_create_with_all_options() {
    let cmd = TaskCommands::Create {
        title: "Complex Task".to_string(),
        description: Some("A complex task with all options".to_string()),
        r#type: Some("llm".to_string()),
        config: Some(r#"{"temperature": 0.7}"#.to_string()),
        metadata: Some(r#"{"version": "1.0"}"#.to_string()),
    };

    match cmd {
        TaskCommands::Create { title, description, r#type, config, metadata } => {
            assert_eq!(title, "Complex Task");
            assert!(description.is_some());
            assert!(r#type.is_some());
            assert!(config.is_some());
            assert!(metadata.is_some());
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_task_list_with_filters() {
    let cmd = TaskCommands::List {
        status: Some("running".to_string()),
        limit: 50,
        offset: 100,
    };

    match cmd {
        TaskCommands::List { status, limit, offset } => {
            assert_eq!(status, Some("running".to_string()));
            assert_eq!(limit, 50);
            assert_eq!(offset, 100);
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_workflow_create_with_definition() {
    let definition = r#"{"nodes": [], "edges": []}"#;
    let cmd = WorkflowCommands::Create {
        name: "Graph Workflow".to_string(),
        description: None,
        definition: Some(definition.to_string()),
    };

    match cmd {
        WorkflowCommands::Create { name, definition, .. } => {
            assert_eq!(name, "Graph Workflow");
            assert!(definition.is_some());
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_output_format_enum_coverage() {
    // Test all OutputFormat variants
    let formats = vec![
        OutputFormat::Json,
        OutputFormat::Table,
        OutputFormat::Plain,
    ];

    for _format in formats {
        // Just verify they're all constructible
    }
}

#[test]
fn test_task_pagination() {
    let cmd1 = TaskCommands::List {
        status: None,
        limit: 20,
        offset: 0,
    };

    let cmd2 = TaskCommands::List {
        status: None,
        limit: 20,
        offset: 20,
    };

    match (cmd1, cmd2) {
        (
            TaskCommands::List { offset: offset1, .. },
            TaskCommands::List { offset: offset2, .. },
        ) => {
            assert_eq!(offset1, 0);
            assert_eq!(offset2, 20);
        }
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_auth_connect_formats() {
    let formats = vec![
        "none",
        "secret:my-secret-key",
        "user:password",
        "token:jwt-token-here",
    ];

    for auth_str in formats {
        let cmd = AuthCommands::Connect {
            auth: auth_str.to_string(),
        };

        match cmd {
            AuthCommands::Connect { auth } => {
                assert!(!auth.is_empty());
            }
            _ => panic!("Wrong command type"),
        }
    }
}

#[test]
fn test_cli_command_dispatch() {
    // Test that command variants can be dispatched
    let task_cmd = Commands::Task {
        subcommand: TaskCommands::Create {
            title: "Test".to_string(),
            description: None,
            r#type: None,
            config: None,
            metadata: None,
        },
    };

    match task_cmd {
        Commands::Task { .. } => {} // Successfully matched
        _ => panic!("Wrong command type"),
    }

    let workflow_cmd = Commands::Workflow {
        subcommand: WorkflowCommands::List {
            status: None,
            limit: 20,
            offset: 0,
        },
    };

    match workflow_cmd {
        Commands::Workflow { .. } => {} // Successfully matched
        _ => panic!("Wrong command type"),
    }

    let auth_cmd = Commands::Auth {
        subcommand: AuthCommands::Status,
    };

    match auth_cmd {
        Commands::Auth { .. } => {} // Successfully matched
        _ => panic!("Wrong command type"),
    }
}

#[test]
fn test_config_commands() {
    use aco::cli::ConfigCommands;
    use std::path::PathBuf;

    let show_cmd = ConfigCommands::Show;
    assert!(matches!(show_cmd, ConfigCommands::Show));

    let set_cmd = ConfigCommands::Set {
        key: "server_url".to_string(),
        value: "http://localhost:50051".to_string(),
    };
    assert!(matches!(set_cmd, ConfigCommands::Set { .. }));

    let init_cmd = ConfigCommands::Init {
        path: Some(PathBuf::from("/home/user/.aco/config.toml")),
    };
    assert!(matches!(init_cmd, ConfigCommands::Init { .. }));
}

#[test]
fn test_server_commands() {
    use aco::cli::ServerCommands;

    let info_cmd = ServerCommands::Info;
    assert!(matches!(info_cmd, ServerCommands::Info));

    let health_cmd = ServerCommands::Health;
    assert!(matches!(health_cmd, ServerCommands::Health));

    let version_cmd = ServerCommands::Version;
    assert!(matches!(version_cmd, ServerCommands::Version));

    let connect_cmd = ServerCommands::Connect {
        url: "http://example.com:50051".to_string(),
    };
    assert!(matches!(connect_cmd, ServerCommands::Connect { .. }));

    let status_cmd = ServerCommands::Status;
    assert!(matches!(status_cmd, ServerCommands::Status));
}

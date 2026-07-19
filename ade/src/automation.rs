//! Automation Framework — command palette, desktop commands, automation API, macros.
//!
//! Provides user-friendly command execution, automation API, and macro support.

use alloc::string::String;
use alloc::vec::Vec;

/// Command category for organization in command palette
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandCategory {
    Window,
    Desktop,
    Application,
    System,
    Plugin,
    Custom,
}

/// Command metadata
#[derive(Clone)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: CommandCategory,
    pub keywords: Vec<String>,
    pub enabled: bool,
}

/// Command execution result
#[derive(Clone)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
}

/// Macro action (atomic operation)
#[derive(Clone)]
pub enum MacroAction {
    /// Execute a command
    ExecuteCommand(String),
    /// Wait for duration (ms)
    Wait(u16),
    /// Set variable
    SetVariable(String, String),
    /// Conditional: if variable equals value
    IfEquals(String, String),
    /// Loop: repeat N times
    Loop(u16),
}

/// Macro (sequence of actions)
#[derive(Clone)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub actions: Vec<MacroAction>,
    pub enabled: bool,
}

/// Automation event
#[derive(Clone, Copy, Debug)]
pub enum AutomationEvent {
    /// Command executed
    CommandExecuted(u32),
    /// Command failed
    CommandFailed(u32),
    /// Macro started
    MacroStarted(u32),
    /// Macro finished
    MacroFinished(u32),
}

/// Command Automation Manager
pub struct AutomationManager {
    /// Registered commands
    commands: Vec<Command>,
    /// Registered macros
    macros: Vec<Macro>,
    /// Command execution history
    history: Vec<String>,
    /// Variables for macros
    variables: Vec<(String, String)>,
    /// Pending events
    events: Vec<AutomationEvent>,
}

impl AutomationManager {
    /// Create a new automation manager
    pub fn new() -> Self {
        AutomationManager {
            commands: Vec::new(),
            macros: Vec::new(),
            history: Vec::new(),
            variables: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Register a new command
    pub fn register_command(&mut self, command: Command) -> u32 {
        self.commands.push(command);
        self.commands.len() as u32 - 1
    }

    /// Get command by ID
    pub fn get_command(&self, command_id: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.id == command_id)
    }

    /// Get all commands
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Get commands by category
    pub fn commands_in_category(&self, category: CommandCategory) -> Vec<&Command> {
        self.commands
            .iter()
            .filter(|c| c.category == category && c.enabled)
            .collect()
    }

    /// Search commands by keywords
    pub fn search_commands(&self, query: &str) -> Vec<&Command> {
        let query_lower = query.to_lowercase();
        self.commands
            .iter()
            .filter(|c| {
                c.enabled
                    && (c.name.to_lowercase().contains(&query_lower)
                        || c.description.to_lowercase().contains(&query_lower)
                        || c.keywords
                            .iter()
                            .any(|k| k.to_lowercase().contains(&query_lower)))
            })
            .collect()
    }

    /// Execute a command
    pub fn execute_command(&mut self, command_id: &str) -> CommandResult {
        if let Some(cmd) = self.get_command(command_id) {
            if !cmd.enabled {
                return CommandResult {
                    success: false,
                    message: String::from("Command is disabled"),
                };
            }

            self.history.push(String::from(command_id));
            if self.history.len() > 100 {
                self.history.remove(0);
            }

            CommandResult {
                success: true,
                message: String::from("Command executed"),
            }
        } else {
            CommandResult {
                success: false,
                message: String::from("Command not found"),
            }
        }
    }

    /// Get execution history
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Register a new macro
    pub fn register_macro(&mut self, macro_: Macro) -> u32 {
        self.macros.push(macro_);
        self.macros.len() as u32 - 1
    }

    /// Get macro by ID
    pub fn get_macro(&self, macro_id: &str) -> Option<&Macro> {
        self.macros.iter().find(|m| m.id == macro_id)
    }

    /// Get all macros
    pub fn macros(&self) -> &[Macro] {
        &self.macros
    }

    /// Execute a macro
    pub fn execute_macro(&mut self, macro_id: &str) -> CommandResult {
        if let Some(macro_) = self.get_macro(macro_id) {
            if !macro_.enabled {
                return CommandResult {
                    success: false,
                    message: String::from("Macro is disabled"),
                };
            }

            // Execute macro actions
            for _action in &macro_.actions {
                // TODO: Execute individual actions
            }

            self.events.push(AutomationEvent::MacroFinished(0));

            CommandResult {
                success: true,
                message: String::from("Macro executed"),
            }
        } else {
            CommandResult {
                success: false,
                message: String::from("Macro not found"),
            }
        }
    }

    /// Set variable for macro use
    pub fn set_variable(&mut self, name: &str, value: &str) {
        if let Some(var) = self.variables.iter_mut().find(|(n, _)| n == name) {
            var.1 = String::from(value);
        } else {
            self.variables
                .push((String::from(name), String::from(value)));
        }
    }

    /// Get variable value
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        self.variables
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Enable/disable command
    pub fn set_command_enabled(&mut self, command_id: &str, enabled: bool) -> bool {
        if let Some(cmd) = self.commands.iter_mut().find(|c| c.id == command_id) {
            cmd.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Enable/disable macro
    pub fn set_macro_enabled(&mut self, macro_id: &str, enabled: bool) -> bool {
        if let Some(macro_) = self.macros.iter_mut().find(|m| m.id == macro_id) {
            macro_.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Delete a macro
    pub fn delete_macro(&mut self, macro_id: &str) -> bool {
        if let Some(pos) = self.macros.iter().position(|m| m.id == macro_id) {
            self.macros.remove(pos);
            true
        } else {
            false
        }
    }

    /// Drain pending automation events
    pub fn drain_events(&mut self) -> Vec<AutomationEvent> {
        let events = self.events.clone();
        self.events.clear();
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_manager_creation() {
        let am = AutomationManager::new();
        assert_eq!(am.commands().len(), 0);
    }

    #[test]
    fn test_register_command() {
        let mut am = AutomationManager::new();
        let cmd = Command {
            id: String::from("test-cmd"),
            name: String::from("Test Command"),
            description: String::from("A test command"),
            category: CommandCategory::System,
            keywords: alloc::vec![String::from("test")],
            enabled: true,
        };
        am.register_command(cmd);
        assert_eq!(am.commands().len(), 1);
    }

    #[test]
    fn test_search_commands() {
        let mut am = AutomationManager::new();
        let cmd = Command {
            id: String::from("close-window"),
            name: String::from("Close Window"),
            description: String::from("Close active window"),
            category: CommandCategory::Window,
            keywords: alloc::vec![String::from("close"), String::from("window")],
            enabled: true,
        };
        am.register_command(cmd);
        let results = am.search_commands("close");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_execute_command() {
        let mut am = AutomationManager::new();
        let cmd = Command {
            id: String::from("test"),
            name: String::from("Test"),
            description: String::from("Test"),
            category: CommandCategory::System,
            keywords: Vec::new(),
            enabled: true,
        };
        am.register_command(cmd);
        let result = am.execute_command("test");
        assert!(result.success);
        assert_eq!(am.history().len(), 1);
    }

    #[test]
    fn test_variables() {
        let mut am = AutomationManager::new();
        am.set_variable("foo", "bar");
        assert_eq!(am.get_variable("foo"), Some("bar"));
    }
}

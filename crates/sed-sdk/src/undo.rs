//! Undo/redo system for SED documents.
//!
//! Every mutation is wrapped in a Command that knows how to undo itself.
//! Commands are stored on a stack. Undo pops the stack and reverses.
//! Redo pushes undone commands back.

use anyhow::Result;
use crate::document::SedDocument;

#[derive(Debug, Clone)]
pub enum Command {
    UpdateField {
        table: String,
        id: String,
        field: String,
        old_value: Option<String>,
        new_value: Option<String>,
    },
    InsertRow {
        table: String,
        id: String,
        /// SQL to recreate this row
        insert_sql: String,
    },
    DeleteRow {
        table: String,
        id: String,
        /// SQL to recreate the deleted row
        restore_sql: String,
    },
}

impl Command {
    pub fn description(&self) -> String {
        match self {
            Command::UpdateField { table, field, .. } => format!("Update {}.{}", table, field),
            Command::InsertRow { table, .. } => format!("Insert into {}", table),
            Command::DeleteRow { table, .. } => format!("Delete from {}", table),
        }
    }
}

pub struct UndoStack {
    undo_stack: Vec<Command>,
    redo_stack: Vec<Command>,
}

impl UndoStack {
    pub fn new() -> Self {
        UndoStack {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, cmd: Command) {
        self.undo_stack.push(cmd);
        self.redo_stack.clear(); // new action invalidates redo history
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self, doc: &SedDocument) -> Result<Option<String>> {
        if let Some(cmd) = self.undo_stack.pop() {
            let desc = cmd.description();
            apply_reverse(&cmd, doc)?;
            self.redo_stack.push(cmd);
            Ok(Some(desc))
        } else {
            Ok(None)
        }
    }

    pub fn redo(&mut self, doc: &SedDocument) -> Result<Option<String>> {
        if let Some(cmd) = self.redo_stack.pop() {
            let desc = cmd.description();
            apply_forward(&cmd, doc)?;
            self.undo_stack.push(cmd);
            Ok(Some(desc))
        } else {
            Ok(None)
        }
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

fn apply_forward(cmd: &Command, doc: &SedDocument) -> Result<()> {
    match cmd {
        Command::UpdateField { table, id, field, new_value, .. } => {
            let sql = format!("UPDATE {} SET {} = ?1 WHERE id = ?2", table, field);
            doc.execute_raw(&sql, &[&new_value as &dyn rusqlite::types::ToSql, &id])?;
        }
        Command::InsertRow { insert_sql, .. } => {
            doc.execute_raw(insert_sql, &[])?;
        }
        Command::DeleteRow { table, id, .. } => {
            let sql = format!("DELETE FROM {} WHERE id = ?1", table);
            doc.execute_raw(&sql, &[&id as &dyn rusqlite::types::ToSql])?;
        }
    }
    Ok(())
}

fn apply_reverse(cmd: &Command, doc: &SedDocument) -> Result<()> {
    match cmd {
        Command::UpdateField { table, id, field, old_value, .. } => {
            let sql = format!("UPDATE {} SET {} = ?1 WHERE id = ?2", table, field);
            doc.execute_raw(&sql, &[&old_value as &dyn rusqlite::types::ToSql, &id])?;
        }
        Command::InsertRow { table, id, .. } => {
            let sql = format!("DELETE FROM {} WHERE id = ?1", table);
            doc.execute_raw(&sql, &[&id as &dyn rusqlite::types::ToSql])?;
        }
        Command::DeleteRow { restore_sql, .. } => {
            doc.execute_raw(restore_sql, &[])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::generate_id;
    use crate::types::*;

    #[test]
    fn undo_redo_update() {
        let doc = SedDocument::in_memory().unwrap();
        let mut stack = UndoStack::new();

        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "L1-01".into(), name: "Room A".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None,
            x: None, y: None,
        }).unwrap();

        // Update name
        doc.update_space(&id, "name", Some("Room B")).unwrap();
        stack.push(Command::UpdateField {
            table: "spaces".into(), id: id.clone(), field: "name".into(),
            old_value: Some("Room A".into()), new_value: Some("Room B".into()),
        });

        let space = doc.get_space(&id).unwrap().unwrap();
        assert_eq!(space.name, "Room B");

        // Undo
        let desc = stack.undo(&doc).unwrap();
        assert_eq!(desc, Some("Update spaces.name".into()));
        let space = doc.get_space(&id).unwrap().unwrap();
        assert_eq!(space.name, "Room A");

        // Redo
        let desc = stack.redo(&doc).unwrap();
        assert_eq!(desc, Some("Update spaces.name".into()));
        let space = doc.get_space(&id).unwrap().unwrap();
        assert_eq!(space.name, "Room B");
    }

    #[test]
    fn undo_redo_counts() {
        let doc = SedDocument::in_memory().unwrap();
        let mut stack = UndoStack::new();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "X1".into(), name: "Test".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None, x: None, y: None,
        }).unwrap();

        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        stack.push(Command::UpdateField {
            table: "spaces".into(), id: id.clone(), field: "name".into(),
            old_value: Some("Test".into()), new_value: Some("Changed".into()),
        });

        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_count(), 1);

        stack.undo(&doc).unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
        assert_eq!(stack.redo_count(), 1);
    }

    #[test]
    fn new_action_clears_redo() {
        let doc = SedDocument::in_memory().unwrap();
        let mut stack = UndoStack::new();
        let id = generate_id();
        doc.add_space(&Space {
            id: id.clone(), tag: "X2".into(), name: "Test".into(),
            level: "Level 1".into(), space_type: None, area_m2: None, ceiling_ht_m: None,
            scope: "in_contract".into(), parent_id: None, boundary_id: None, x: None, y: None,
        }).unwrap();

        stack.push(Command::UpdateField {
            table: "spaces".into(), id: id.clone(), field: "name".into(),
            old_value: Some("Test".into()), new_value: Some("A".into()),
        });
        stack.undo(&doc).unwrap();
        assert!(stack.can_redo());

        stack.push(Command::UpdateField {
            table: "spaces".into(), id: id.clone(), field: "name".into(),
            old_value: Some("Test".into()), new_value: Some("B".into()),
        });
        assert!(!stack.can_redo());
    }
}

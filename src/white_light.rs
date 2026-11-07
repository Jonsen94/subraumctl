//! Known white-light cells and their labels.
//!
//! The web UI grouped the cells into `hack` and `make` columns; that grouping
//! is preserved here for display purposes only.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CellGroup {
    Hack,
    Make,
}

impl CellGroup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hack => "hack",
            Self::Make => "make",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Cell {
    pub id: &'static str,
    pub group: CellGroup,
}

pub const CELLS: [Cell; 15] = [
    Cell {
        id: "LM3_A020A63498E7",
        group: CellGroup::Hack,
    },
    Cell {
        id: "LM3_A020A6349898",
        group: CellGroup::Hack,
    },
    Cell {
        id: "LM3_A020A6349701",
        group: CellGroup::Hack,
    },
    Cell {
        id: "LM3_A020A634988B",
        group: CellGroup::Hack,
    },
    Cell {
        id: "LM3_A020A6167801",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_18FE34FE4C87",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_5CCF7F01C9DF",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_A020A6349804",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_A020A6349BE3",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_A020A61849EF",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_A020A6167E67",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_5CCF7F01CF21",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_18FE34FE48FC",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_A020A6349BF4",
        group: CellGroup::Make,
    },
    Cell {
        id: "LM3_A020A6349910",
        group: CellGroup::Make,
    },
];

pub fn find(id: &str) -> Option<&'static Cell> {
    CELLS.iter().find(|cell| cell.id == id)
}

pub fn index_of(id: &str) -> Option<usize> {
    CELLS.iter().position(|cell| cell.id == id)
}

pub fn ids() -> impl Iterator<Item = &'static str> {
    CELLS.iter().map(|cell| cell.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_cells_are_unique() {
        let unique: HashSet<_> = ids().collect();
        assert_eq!(unique.len(), CELLS.len());
    }

    #[test]
    fn groups_match_the_web_ui() {
        let hack = CELLS
            .iter()
            .filter(|cell| cell.group == CellGroup::Hack)
            .count();
        let make = CELLS
            .iter()
            .filter(|cell| cell.group == CellGroup::Make)
            .count();
        assert_eq!(hack, 4);
        assert_eq!(make, 11);
    }

    #[test]
    fn lookups_work() {
        assert_eq!(index_of("LM3_A020A63498E7"), Some(0));
        assert_eq!(find("LM3_A020A6349910").unwrap().group, CellGroup::Make);
        assert!(find("nope").is_none());
    }
}

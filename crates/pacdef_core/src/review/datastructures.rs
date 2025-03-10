use std::rc::Rc;

use crate::backend::Backend;
use crate::{Group, Package};

use super::strategy::Strategy;

#[derive(Debug, PartialEq)]
pub(super) enum ReviewAction {
    AsDependency(Package),
    Delete(Package),
    AssignGroup(Package, Rc<Group>),
}

#[derive(Debug)]
pub(super) enum ReviewIntention {
    AsDependency,
    AssignGroup,
    Delete,
    Info,
    Invalid,
    Skip,
    Quit,
}

#[derive(Debug)]
pub(super) struct ReviewsPerBackend {
    items: Vec<(Box<dyn Backend>, Vec<ReviewAction>)>,
}

impl ReviewsPerBackend {
    pub(super) fn new() -> Self {
        Self { items: vec![] }
    }

    pub(super) fn nothing_to_do(&self) -> bool {
        self.items.iter().all(|(_, vec)| vec.is_empty())
    }

    pub(super) fn push(&mut self, value: (Box<dyn Backend>, Vec<ReviewAction>)) {
        self.items.push(value);
    }

    /// Convert the reviews per backend to a vector of [`Strategy`], where one `Strategy` contains
    /// all actions that must be executed for a [`Backend`].
    ///
    /// If there are no actions for a `Backend`, then that `Backend` is removed from the return
    /// value.
    pub(super) fn into_strategies(self) -> Vec<Strategy> {
        let mut result = vec![];

        for (backend, actions) in self {
            let mut to_delete = vec![];
            let mut assign_group = vec![];
            let mut as_dependency = vec![];

            extract_actions(
                actions,
                &mut to_delete,
                &mut assign_group,
                &mut as_dependency,
            );

            result.push(Strategy::new(
                backend,
                to_delete,
                as_dependency,
                assign_group,
            ));
        }

        result.retain(|s| !s.nothing_to_do());

        result
    }
}

impl IntoIterator for ReviewsPerBackend {
    type Item = (Box<dyn Backend>, Vec<ReviewAction>);

    type IntoIter = std::vec::IntoIter<(Box<dyn Backend>, Vec<ReviewAction>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

pub(super) enum ContinueWithReview {
    Yes,
    No,
}

fn extract_actions(
    actions: Vec<ReviewAction>,
    to_delete: &mut Vec<Package>,
    assign_group: &mut Vec<(Package, Rc<Group>)>,
    as_dependency: &mut Vec<Package>,
) {
    for action in actions {
        match action {
            ReviewAction::Delete(package) => to_delete.push(package),
            ReviewAction::AssignGroup(package, group) => assign_group.push((package, group)),
            ReviewAction::AsDependency(package) => as_dependency.push(package),
        }
    }
}

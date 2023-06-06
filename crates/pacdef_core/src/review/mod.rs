mod datastructures;
mod strategy;
mod ui;

use std::io::{stdin, stdout, Write};
use std::rc::Rc;

use anyhow::Result;

use crate::backend::{Backend, ToDoPerBackend};
use crate::ui::{get_user_confirmation, read_single_char_from_terminal};
use crate::{Group, Package};

use self::datastructures::{ContinueWithReview, ReviewAction, ReviewIntention, ReviewsPerBackend};
use self::strategy::Strategy;

pub fn review(
    todo_per_backend: ToDoPerBackend,
    groups: impl IntoIterator<Item = Group>,
) -> Result<()> {
    let mut reviews = ReviewsPerBackend::new();
    let mut groups: Vec<Rc<Group>> = groups.into_iter().map(Rc::new).collect();

    groups.sort_unstable();

    if todo_per_backend.nothing_to_do_for_all_backends() {
        println!("nothing to do");
        return Ok(());
    }

    for (backend, packages) in todo_per_backend.into_iter() {
        let mut actions = vec![];
        for package in packages {
            println!("{}: {package}", backend.get_section());
            match get_action_for_package(package, &groups, &mut actions, &*backend)? {
                ContinueWithReview::Yes => continue,
                ContinueWithReview::No => return Ok(()),
            }
        }
        reviews.push((backend, actions));
    }

    if reviews.nothing_to_do() {
        println!("nothing to do");
        return Ok(());
    }

    let strategies: Vec<Strategy> = reviews.into_strategies();

    println!();
    let mut iter = strategies.iter().peekable();

    while let Some(strat) = iter.next() {
        strat.show();

        if iter.peek().is_some() {
            println!();
        }
    }

    println!();
    if !get_user_confirmation()? {
        return Ok(());
    }

    for strat in strategies {
        strat.execute()?;
    }

    Ok(())
}

fn get_action_for_package(
    package: Package,
    groups: &[Rc<Group>],
    reviews: &mut Vec<ReviewAction>,
    backend: &dyn Backend,
) -> Result<ContinueWithReview> {
    loop {
        match ask_user_action_for_package(backend.supports_as_dependency())? {
            ReviewIntention::AsDependency => {
                assert!(
                    backend.supports_as_dependency(),
                    "backend does not support dependencies"
                );
                reviews.push(ReviewAction::AsDependency(package));
                break;
            }
            ReviewIntention::AssignGroup => {
                if let Ok(Some(group)) = ask_group(groups) {
                    reviews.push(ReviewAction::AssignGroup(package, group));
                    break;
                };
            }
            ReviewIntention::Delete => {
                reviews.push(ReviewAction::Delete(package));
                break;
            }
            ReviewIntention::Info => {
                backend.show_package_info(&package)?;
            }
            ReviewIntention::Invalid => (),
            ReviewIntention::Skip => break,
            ReviewIntention::Quit => return Ok(ContinueWithReview::No),
        }
    }
    Ok(ContinueWithReview::Yes)
}

/// Ask the user for the desired action, and return the associated
/// [`ReviewIntention`]. The query depends on the capabilities of the backend.
///
/// # Errors
///
/// This function will return an error if stdin or stdout cannot be accessed.
fn ask_user_action_for_package(supports_as_dependency: bool) -> Result<ReviewIntention> {
    print_query(supports_as_dependency)?;

    match read_single_char_from_terminal()?.to_ascii_lowercase() {
        'a' if supports_as_dependency => Ok(ReviewIntention::AsDependency),
        'd' => Ok(ReviewIntention::Delete),
        'g' => Ok(ReviewIntention::AssignGroup),
        'i' => Ok(ReviewIntention::Info),
        'q' => Ok(ReviewIntention::Quit),
        's' => Ok(ReviewIntention::Skip),
        _ => Ok(ReviewIntention::Invalid),
    }
}

/// Print a space-terminated string that asks the user for the desired action.
/// The items of the string depend on whether the backend supports dependent
/// packages.
///
/// # Errors
///
/// This function will return an error if stdout cannot be flushed.
fn print_query(supports_as_dependency: bool) -> Result<()> {
    let mut query = String::from("assign to (g)roup, (d)elete, (s)kip, (i)nfo, ");

    if supports_as_dependency {
        query.push_str("(a)s dependency, ");
    }

    query.push_str("(q)uit? ");

    print!("{query}");
    stdout().lock().flush()?;
    Ok(())
}

fn print_enumerated_groups(groups: &[Rc<Group>]) {
    let number_digits = get_amount_of_digits_for_number(groups.len());

    for (i, group) in groups.iter().enumerate() {
        println!("{i:>number_digits$}: {}", group.name);
    }
}

#[allow(clippy::as_conversions)] // this cannot introduce errors for any reasonably sized numbers.
fn get_amount_of_digits_for_number(number: usize) -> usize {
    (number as f64).log10().trunc() as usize + 1
}

fn ask_group(groups: &[Rc<Group>]) -> Result<Option<Rc<Group>>> {
    print_enumerated_groups(groups);
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;
    let reply = buf.trim();

    let idx: usize = if let Ok(idx) = reply.parse() {
        idx
    } else {
        return Ok(None);
    };

    if idx < groups.len() {
        Ok(Some(groups[idx].clone()))
    } else {
        Ok(None)
    }
}

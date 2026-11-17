//! What an Execute action runs, from the text, the caret, the selection and
//! the options of the action (Settings > Tools > Database > Query
//! Execution).

use std::ops::Range;

use sql_syntax::{CandidateKind, Dialect, Statement};

use crate::query_settings::{CaretInside, CaretOutside, ExecuteOptions, ForSelection};

/// One answer to "what to execute".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Choice {
    pub label: String,
    pub statements: Vec<Statement>,
}

/// The outcome of the options applied to the editor's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Plan {
    /// Nothing to run (no statements, or the options say so).
    Nothing,
    Run(Vec<Statement>),
    /// More than one thing could run and the options ask: the person picks.
    Ask {
        choices: Vec<Choice>,
        selected: usize,
    },
}

/// The longest label of a choice, in characters.
const LABEL_CHARS: usize = 70;

/// `caret` and `selection` are byte offsets into `text`.
pub(crate) fn plan(
    text: &str,
    caret: usize,
    selection: Range<usize>,
    dialect: Dialect,
    options: &ExecuteOptions,
) -> Plan {
    if !selection.is_empty() {
        let statements = match options.for_selection {
            ForSelection::Separate => sql_syntax::statements_in(text, selection, dialect),
            ForSelection::SmartExpand => smart_expand(text, selection, dialect),
            ForSelection::One => sql_syntax::statement_of(text, selection, dialect)
                .into_iter()
                .collect(),
        };
        return run(statements);
    }
    let statements = sql_syntax::split(text, dialect);
    // The statement's own `;` is part of it for the caret.
    let index = statements
        .iter()
        .rposition(|s| s.range.start <= caret)
        .filter(|&i| {
            let statement = &statements[i];
            let end = statement.range.end;
            if caret > end {
                return caret == end + 1 && text.as_bytes().get(end) == Some(&b';');
            }
            // Comments within a statement use the same outside policy as a standalone comment.
            let local = caret - statement.range.start;
            !((statement.text.contains("--") || statement.text.contains("/*"))
                && sql_syntax::lex(&statement.text, dialect)
                    .iter()
                    .any(|token| {
                        matches!(
                            token.kind,
                            sql_syntax::TokenKind::LineComment
                                | sql_syntax::TokenKind::BlockComment
                        ) && token.range().contains(&local)
                    }))
        });
    if let Some(index) = index {
        let statement = &statements[index];
        return match options.caret_inside {
            CaretInside::WholeScript => run(statements),
            CaretInside::FromCaret => run(statements.into_iter().skip(index)),
            // The supported dialects have no Transact-SQL batch separator.
            CaretInside::Largest | CaretInside::LargestOrBatch => run([statement.clone()]),
            CaretInside::SmallestStatement => {
                let nested = nested_statement(statement, caret, dialect)
                    .and_then(|r| sql_syntax::statement_of(text, r, dialect));
                run([nested.unwrap_or_else(|| statement.clone())])
            }
            CaretInside::Ask | CaretInside::Smallest => {
                let mut ranges: Vec<_> = sql_syntax::candidates_at(text, caret, dialect)
                    .into_iter()
                    .filter(|c| c.kind != CandidateKind::Script)
                    .map(|c| c.range)
                    .chain(nested_statement(statement, caret, dialect))
                    .collect();
                ranges.sort_by_key(Range::len);
                ranges.dedup();
                let mut scoped = ranges
                    .into_iter()
                    .filter_map(|range| sql_syntax::statement_of(text, range, dialect));
                if options.caret_inside == CaretInside::Smallest {
                    run(scoped.next())
                } else {
                    let mut choices: Vec<_> = scoped.map(choice).collect();
                    let selected = choices.len().saturating_sub(1);
                    if statements.len() > 1 {
                        choices.push(Choice {
                            label: preview(text),
                            statements,
                        });
                    }
                    ask_or_run(choices, selected)
                }
            }
        };
    }
    match options.caret_outside {
        CaretOutside::Nothing => Plan::Nothing,
        CaretOutside::WholeScript => run(statements),
        CaretOutside::BelowCaret => run(statements.into_iter().filter(|s| s.range.start >= caret)),
        CaretOutside::Closest => run(sql_syntax::statement_at(text, caret, dialect)),
        CaretOutside::Ask => {
            let mut choices: Vec<_> = sql_syntax::statement_at(text, caret, dialect)
                .map(choice)
                .into_iter()
                .collect();
            if statements.len() > 1 {
                choices.push(Choice {
                    label: "All statements".to_owned(),
                    statements,
                });
            }
            ask_or_run(choices, 0)
        }
    }
}

/// Procedural bodies can contain complete statements of their own.
fn nested_statement(statement: &Statement, caret: usize, dialect: Dialect) -> Option<Range<usize>> {
    if !statement.text.contains(';') {
        return None;
    }
    let start = statement.range.start;
    let parsed = sql_syntax::parse(&statement.text, dialect);
    let leaf = parsed.node_at(caret - start)?;
    let node = std::iter::once(leaf).chain(leaf.ancestors()).find(|n| {
        n.parent()
            .is_some_and(|p| p.kind() == sql_syntax::SyntaxKind::StatementList)
    })?;
    let range = node.range();
    Some(start + range.start..start + range.end)
}

fn smart_expand(text: &str, selection: Range<usize>, dialect: Dialect) -> Vec<Statement> {
    let touched: Vec<_> = sql_syntax::split(text, dialect)
        .into_iter()
        .filter(|s| s.range.start < selection.end && selection.start < s.range.end)
        .collect();
    if touched
        .iter()
        .any(|s| selection.start < s.range.start || s.range.end < selection.end)
    {
        touched
    } else {
        sql_syntax::statements_in(text, selection, dialect)
    }
}

fn run(statements: impl IntoIterator<Item = Statement>) -> Plan {
    let statements: Vec<Statement> = statements.into_iter().collect();
    if statements.is_empty() {
        Plan::Nothing
    } else {
        Plan::Run(statements)
    }
}

fn ask_or_run(mut choices: Vec<Choice>, selected: usize) -> Plan {
    match choices.len() {
        0 => Plan::Nothing,
        1 => run(choices.remove(0).statements),
        _ => Plan::Ask { choices, selected },
    }
}

fn choice(statement: Statement) -> Choice {
    Choice {
        label: preview(&statement.text),
        statements: vec![statement],
    }
}

fn preview(text: &str) -> String {
    let mut chars = text
        .split_whitespace()
        .enumerate()
        .flat_map(|(i, word)| (i > 0).then_some(' ').into_iter().chain(word.chars()));
    let mut label: String = chars.by_ref().take(LABEL_CHARS).collect();
    if chars.next().is_some() {
        label.pop();
        label.push('…');
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    const PG: Dialect = Dialect::Postgres;

    fn options(inside: CaretInside, outside: CaretOutside, sel: ForSelection) -> ExecuteOptions {
        ExecuteOptions {
            caret_inside: inside,
            caret_outside: outside,
            for_selection: sel,
            new_tab: false,
        }
    }

    fn at(script: &str, marker: &str, offset: usize, o: &ExecuteOptions) -> Plan {
        let caret = script.find(marker).unwrap() + offset;
        plan(script, caret, caret..caret, PG, o)
    }

    fn texts(plan: &Plan) -> Vec<String> {
        match plan {
            Plan::Run(s) => s.iter().map(|s| s.text.clone()).collect(),
            Plan::Ask { choices, .. } => choices.iter().map(|c| c.label.clone()).collect(),
            Plan::Nothing => Vec::new(),
        }
    }

    const SCRIPT: &str = "select 1;\n\nselect * from (select a from t) x;\n\n\n\nselect 3;";

    #[test]
    fn ask_runs_a_plain_statement_at_once() {
        let o = ExecuteOptions::default();
        let p = at("select 1;", "select 1", 3, &o);
        assert_eq!(texts(&p), ["select 1"]);
        assert!(matches!(p, Plan::Run(_)));
    }

    #[test]
    fn ask_asks_inside_a_subquery() {
        let o = ExecuteOptions::default();
        let p = at(SCRIPT, "select a", 3, &o);
        assert!(matches!(p, Plan::Ask { selected: 1, .. }));
        assert_eq!(
            texts(&p),
            [
                "select a from t",
                "select * from (select a from t) x",
                "select 1; select * from (select a from t) x; select 3;"
            ]
        );
    }

    #[test]
    fn ask_deduplicates_a_single_script_and_preserves_statement_boundaries() {
        let o = ExecuteOptions::default();
        let Plan::Ask { choices, selected } = at("select (select 1);", "select 1", 3, &o) else {
            panic!("a subquery and its surrounding statement");
        };
        assert_eq!(choices.len(), 2);
        assert_eq!(selected, 1);
        let Plan::Ask { choices, selected } = at("select 'a;b';select 2;", "select 2", 3, &o)
        else {
            panic!("a statement and the script");
        };
        assert_eq!(selected, 0);
        assert_eq!(choices.len(), 2);
        assert_eq!(
            choices[1]
                .statements
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            ["select 'a;b'", "select 2"]
        );
        assert!(matches!(at("select 1;", "select 1", 3, &o), Plan::Run(_)));
    }

    #[test]
    fn smallest_and_largest() {
        let smallest = options(
            CaretInside::Smallest,
            CaretOutside::Nothing,
            ForSelection::Separate,
        );
        let largest = options(
            CaretInside::Largest,
            CaretOutside::Nothing,
            ForSelection::Separate,
        );
        assert_eq!(
            texts(&at(SCRIPT, "select a", 3, &smallest)),
            ["select a from t"]
        );
        assert_eq!(
            texts(&at(SCRIPT, "select a", 3, &largest)),
            ["select * from (select a from t) x"]
        );
    }

    #[test]
    fn outside_a_statement() {
        // The gap of blank lines between the second and the third statement.
        let caret = SCRIPT.find("x;").unwrap() + 4;
        let blank = |o: &ExecuteOptions| plan(SCRIPT, caret, caret..caret, PG, o);
        let nothing = options(
            CaretInside::Ask,
            CaretOutside::Nothing,
            ForSelection::Separate,
        );
        assert_eq!(blank(&nothing), Plan::Nothing);
        let closest = options(
            CaretInside::Ask,
            CaretOutside::Closest,
            ForSelection::Separate,
        );
        assert_eq!(
            texts(&blank(&closest)),
            ["select * from (select a from t) x"]
        );
        let ask = options(CaretInside::Ask, CaretOutside::Ask, ForSelection::Separate);
        assert_eq!(
            texts(&blank(&ask)),
            ["select * from (select a from t) x", "All statements"]
        );
    }

    #[test]
    fn a_selection_runs_as_separate_statements_or_as_one() {
        let from = SCRIPT.find("select 1").unwrap();
        let to = SCRIPT.find("select 3").unwrap();
        let separate = ExecuteOptions::default();
        let p = plan(SCRIPT, to, from..to, PG, &separate);
        assert_eq!(texts(&p), ["select 1", "select * from (select a from t) x"]);
        let one = options(CaretInside::Ask, CaretOutside::Nothing, ForSelection::One);
        let p = plan(SCRIPT, to, from..to, PG, &one);
        assert_eq!(
            texts(&p),
            ["select 1;\n\nselect * from (select a from t) x"]
        );
    }

    #[test]
    fn script_and_tail_keep_complete_statements_in_source_order() {
        let input = "select 1;select (select 2);\n\nselect 3;";
        let mut o = ExecuteOptions::default();
        let all = ["select 1", "select (select 2)", "select 3"];
        o.caret_inside = CaretInside::WholeScript;
        assert_eq!(texts(&at(input, "select 2", 4, &o)), all);
        o.caret_inside = CaretInside::FromCaret;
        for (marker, offset) in [("select (", 0), ("select 2", 4)] {
            assert_eq!(texts(&at(input, marker, offset, &o)), all[1..]);
        }
        o.caret_outside = CaretOutside::WholeScript;
        assert_eq!(texts(&at(input, "\n\n", 1, &o)), all);
        o.caret_outside = CaretOutside::BelowCaret;
        assert_eq!(texts(&at(input, "\n\n", 1, &o)), ["select 3"]);
        assert_eq!(at("select 1;\n", "\n", 1, &o), Plan::Nothing);
        assert_eq!(
            texts(&at("select 1; -- here\n select 2;", "here", 1, &o)),
            ["select 2"]
        );
        assert_eq!(
            texts(&at("select /* here */ 1; select 2;", "here", 1, &o)),
            ["select 2"]
        );
    }

    #[test]
    fn smallest_statement_excludes_subqueries_and_can_run_a_trigger_statement() {
        let mut o = ExecuteOptions {
            caret_inside: CaretInside::SmallestStatement,
            ..Default::default()
        };
        let input = "select 1; select (select 2);";
        assert_eq!(texts(&at(input, "select 2", 4, &o)), ["select (select 2)"]);
        let trigger = "create trigger log after insert on t begin select 41; select 42; end;";
        let caret = trigger.find("42").unwrap();
        assert_eq!(
            texts(&plan(trigger, caret, caret..caret, Dialect::Sqlite, &o)),
            ["select 42"]
        );
        o.caret_inside = CaretInside::LargestOrBatch;
        assert_eq!(
            texts(&plan(trigger, caret, caret..caret, Dialect::Sqlite, &o)),
            [trigger.trim_end_matches(';')]
        );
    }

    #[test]
    fn smart_selection_expands_only_when_crossing_a_statement_boundary() {
        let input = "select 'a;b'; select (select 22); select 33;";
        let o = ExecuteOptions {
            for_selection: ForSelection::SmartExpand,
            ..Default::default()
        };
        let nested = input.find("select 22").unwrap();
        let run = |range: Range<usize>| texts(&plan(input, range.end, range, PG, &o));
        assert_eq!(run(nested..nested + 9), ["select 22"]);
        assert_eq!(run(8..nested + 1), ["select 'a;b'", "select (select 22)"]);
        assert!(run(12..14).is_empty());
    }

    #[test]
    fn an_empty_script_runs_nothing() {
        let o = ExecuteOptions::default();
        assert_eq!(plan("", 0, 0..0, PG, &o), Plan::Nothing);
        assert_eq!(plan("-- x", 2, 2..2, PG, &o), Plan::Nothing);
        for mode in [
            ForSelection::One,
            ForSelection::Separate,
            ForSelection::SmartExpand,
        ] {
            let options = ExecuteOptions {
                for_selection: mode,
                ..o
            };
            for empty in ["-- x", "; ;", " /* ; */ ; \n; -- x"] {
                assert_eq!(
                    plan(empty, empty.len(), 0..empty.len(), PG, &options),
                    Plan::Nothing
                );
            }
        }
    }
}

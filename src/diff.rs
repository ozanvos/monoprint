//! Unified, syntax-preserving changes built from versions, patches or line marks.
use crate::{Error, InvalidOption, Language, LineRange, MAX_SOURCE_BYTES, Renderer, syntax};
use similar::{Algorithm, ChangeTag, TextDiff};
use std::{ops::Range, time::Duration};
use unicode_segmentation::UnicodeSegmentation;

/// Number of unchanged lines retained around a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    /// Retain all known lines.
    All,
    /// Retain this many lines on each side.
    Lines(usize),
}
impl Default for Context {
    fn default() -> Self {
        Self::Lines(3)
    }
}
/// Gutter number selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffNumbers {
    /// New version numbers.
    #[default]
    New,
    /// Old version numbers.
    Old,
    /// Both versions.
    Both,
}
impl DiffNumbers {
    pub(crate) fn largest(
        self,
        rows: impl IntoIterator<Item = impl std::borrow::Borrow<Row>>,
    ) -> usize {
        rows.into_iter()
            .flat_map(|row| {
                let row = row.borrow();
                match self {
                    Self::New => [row.new, None],
                    Self::Old => [row.old, None],
                    Self::Both => [row.old, row.new],
                }
            })
            .flatten()
            .max()
            .unwrap_or(1)
    }
}
impl std::str::FromStr for DiffNumbers {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Error> {
        match s {
            "new" => Ok(Self::New),
            "old" => Ok(Self::Old),
            "both" => Ok(Self::Both),
            _ => Err(invalid("diff numbers must be new, old, or both")),
        }
    }
}
/// Role of a displayed row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// Unchanged code.
    Context,
    /// Inserted code.
    Added,
    /// Deleted code.
    Removed,
    /// Omitted context.
    Fold {
        /// Number of omitted lines.
        hidden: usize,
    },
}
/// One row in a unified change.
#[derive(Clone, Debug)]
pub struct Row {
    /// Change classification.
    pub kind: RowKind,
    /// Old source number.
    pub old: Option<usize>,
    /// New source number.
    pub new: Option<usize>,
    pub(crate) text: String,
    side_line: usize,
}
/// An owned change. Setters are validated when used by a renderer.
#[derive(Clone, Debug)]
pub struct Diff {
    old: String,
    new: String,
    rows: Vec<Row>,
    context: Context,
    marked: bool,
    added: Vec<LineRange>,
    removed: Vec<LineRange>,
    first: usize,
    path: Option<String>,
    patch_source: Option<String>,
    error: Option<String>,
}
fn invalid(message: impl Into<String>) -> Error {
    InvalidOption::Diff(message.into()).into()
}
fn normalize(s: &str) -> Result<String, Error> {
    if s.len() > MAX_SOURCE_BYTES {
        return Err(InvalidOption::SourceTooLarge { len: s.len() }.into());
    }
    Ok(s.replace("\r\n", "\n").replace('\r', "\n"))
}
impl Diff {
    /// Serialize a comparison as a self-contained unified patch.
    pub fn patch(&self) -> String {
        if let Some(source) = &self.patch_source {
            return source.clone();
        }
        if self.old == self.new {
            let count = self.new.lines().count().max(1);
            let mut patch = format!("--- a/source\n+++ b/source\n@@ -1,{count} +1,{count} @@\n");
            if self.new.is_empty() {
                patch.push_str(" \n\\ No newline at end of file\n");
            }
            for line in self.new.lines() {
                patch.push(' ');
                patch.push_str(line);
                patch.push('\n');
            }
            if !self.new.ends_with('\n') && !self.new.is_empty() {
                patch.push_str("\\ No newline at end of file\n");
            }
            return patch;
        }
        TextDiff::configure()
            .algorithm(Algorithm::Patience)
            .timeout(Duration::from_secs(2))
            .diff_lines(&self.old, &self.new)
            .unified_diff()
            .context_radius(self.old.lines().count().max(self.new.lines().count()))
            .header("a/source", "b/source")
            .to_string()
    }

    /// Compare complete versions, highlighting each independently.
    pub fn compare(old: &str, new: &str) -> Result<Self, Error> {
        let old = normalize(old)?;
        let new = normalize(new)?;
        let rows = TextDiff::configure()
            .algorithm(Algorithm::Patience)
            .timeout(Duration::from_secs(2))
            .diff_lines(&old, &new)
            .iter_all_changes()
            .map(|c| Row {
                kind: match c.tag() {
                    ChangeTag::Equal => RowKind::Context,
                    ChangeTag::Delete => RowKind::Removed,
                    ChangeTag::Insert => RowKind::Added,
                },
                old: c.old_index().map(|n| n + 1),
                new: c.new_index().map(|n| n + 1),
                text: c.value().trim_end_matches('\n').to_owned(),
                side_line: c.new_index().or(c.old_index()).unwrap(),
            })
            .collect();
        Ok(Self {
            old,
            new,
            rows,
            context: Context::default(),
            marked: false,
            added: vec![],
            removed: vec![],
            first: 1,
            path: None,
            patch_source: None,
            error: None,
        })
    }
    /// Mark a single snippet without comparing its lines.
    pub fn marked(source: &str) -> Self {
        let normalized = normalize(source);
        Self {
            old: String::new(),
            new: normalized.as_ref().cloned().unwrap_or_default(),
            rows: vec![],
            context: Context::All,
            marked: true,
            added: vec![],
            removed: vec![],
            first: 1,
            path: None,
            patch_source: None,
            error: normalized.err().map(|e| e.to_string()),
        }
    }
    /// Mark one or more inserted lines.
    pub fn added(mut self, range: impl Into<LineRange>) -> Self {
        if !self.marked {
            self.error = Some("marks cannot be combined with compare or patch input".into());
        }
        self.added.push(range.into());
        self
    }
    /// Mark one or more deleted lines.
    pub fn removed(mut self, range: impl Into<LineRange>) -> Self {
        if !self.marked {
            self.error = Some("marks cannot be combined with compare or patch input".into());
        }
        self.removed.push(range.into());
        self
    }
    /// Starting number for marked snippets.
    pub fn first_line(mut self, n: usize) -> Self {
        self.first = n;
        self
    }
    /// Set retained context; marked snippets always retain all lines.
    pub fn context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }
    /// File path recorded in a patch, if available.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
    /// Displayed rows, including fold furniture. Invalid marks return an error.
    pub fn rows(&self) -> Result<Vec<Row>, Error> {
        self.display().map(|(rows, _)| rows)
    }
    /// Visible code (fold labels are excluded).
    pub fn displayed_source(&self) -> String {
        self.display().map(|(_, s)| s).unwrap_or_default()
    }
    /// Parse a single-file unified patch with validated hunk counts.
    pub fn from_patch(patch: &str) -> Result<Self, Error> {
        let patch = normalize(patch)?;
        let mut result = Self::compare("", "")?;
        let mut files = Vec::new();
        let mut old_lines = Vec::new();
        let mut new_lines = Vec::new();
        let lines: Vec<_> = patch.lines().collect();
        let mut i = 0;
        let mut previous: Option<(usize, usize)> = None;
        let mut git_files = Vec::new();
        let mut last_kind = None;
        let (mut old_no_newline, mut new_no_newline) = (false, false);
        while i < lines.len() {
            let line = lines[i];
            i += 1;
            if let Some(file) = line.strip_prefix("diff --git ") {
                git_files.push(file.to_owned());
            }
            if line == "\\ No newline at end of file" {
                match last_kind {
                    Some(RowKind::Removed) => old_no_newline = true,
                    Some(RowKind::Added) => new_no_newline = true,
                    Some(RowKind::Context) => {
                        old_no_newline = true;
                        new_no_newline = true;
                    }
                    _ => return Err(invalid("orphan newline marker")),
                }
                continue;
            }
            if let Some(path) = line.strip_prefix("+++ ") {
                let path = path
                    .split('\t')
                    .next()
                    .unwrap()
                    .strip_prefix("b/")
                    .unwrap_or(path.split('\t').next().unwrap())
                    .to_owned();
                files.push(path.clone());
                if path != "/dev/null" {
                    result.path = Some(path);
                }
                continue;
            }
            if let Some(path) = line.strip_prefix("--- ") {
                if result.path.is_none() {
                    result.path = Some(
                        path.split('\t')
                            .next()
                            .unwrap()
                            .strip_prefix("a/")
                            .unwrap_or(path.split('\t').next().unwrap())
                            .into(),
                    );
                }
                continue;
            }
            if !line.starts_with("@@ ") {
                if line.starts_with(['+', '-', ' ']) && previous.is_some() {
                    return Err(invalid("unexpected patch line outside hunk"));
                }
                continue;
            }
            let fields: Vec<_> = line.split_whitespace().collect();
            let parse = |s: &str, sign: char| -> Result<(usize, usize), Error> {
                let s = s
                    .strip_prefix(sign)
                    .ok_or_else(|| invalid("invalid hunk header"))?;
                let (a, b) = s.split_once(',').unwrap_or((s, "1"));
                Ok((
                    a.parse().map_err(|_| invalid("invalid hunk start"))?,
                    b.parse().map_err(|_| invalid("invalid hunk count"))?,
                ))
            };
            if fields.len() < 4 || fields[3] != "@@" {
                return Err(invalid("invalid hunk header"));
            }
            let (mut o, oc) = parse(fields[1], '-')?;
            let (mut n, nc) = parse(fields[2], '+')?;
            if (o == 0 && oc > 0) || (n == 0 && nc > 0) {
                return Err(invalid("nonempty hunk ranges start at one"));
            }
            if oc > MAX_SOURCE_BYTES
                || nc > MAX_SOURCE_BYTES
                || o.checked_add(oc).is_none()
                || n.checked_add(nc).is_none()
            {
                return Err(invalid("hunk range too large"));
            }
            // Empty unified ranges denote the boundary *after* their start.
            // Convert them to the next one-based source position for gap math.
            if oc == 0 {
                o = o
                    .checked_add(1)
                    .ok_or_else(|| invalid("hunk range too large"))?;
            }
            if nc == 0 {
                n = n
                    .checked_add(1)
                    .ok_or_else(|| invalid("hunk range too large"))?;
            }
            if let Some((po, pn)) = previous {
                if o < po || n < pn {
                    return Err(invalid("overlapping patch hunks"));
                }
                let gap = o - po;
                if gap != n - pn {
                    return Err(invalid("inconsistent patch hunk gap"));
                }
                if gap > 0 {
                    result.rows.push(Row {
                        kind: RowKind::Fold { hidden: gap },
                        old: None,
                        new: None,
                        text: String::new(),
                        side_line: 0,
                    });
                }
            }
            let (mut used_o, mut used_n) = (0, 0);
            while used_o < oc || used_n < nc {
                let line = *lines
                    .get(i)
                    .ok_or_else(|| invalid("truncated patch hunk"))?;
                i += 1;
                if line == "\\ No newline at end of file" {
                    match last_kind {
                        Some(RowKind::Removed) => old_no_newline = true,
                        Some(RowKind::Added) => new_no_newline = true,
                        Some(RowKind::Context) => {
                            old_no_newline = true;
                            new_no_newline = true;
                        }
                        _ => return Err(invalid("orphan newline marker")),
                    }
                    continue;
                }
                let (kind, text) = match line.as_bytes().first() {
                    Some(b' ') => (RowKind::Context, &line[1..]),
                    Some(b'-') => (RowKind::Removed, &line[1..]),
                    Some(b'+') => (RowKind::Added, &line[1..]),
                    _ => return Err(invalid("invalid patch hunk line")),
                };
                last_kind = Some(kind);
                let old = kind != RowKind::Added;
                let new = kind != RowKind::Removed;
                if (old && old_no_newline) || (new && new_no_newline) {
                    return Err(invalid("patch content follows an end-of-file marker"));
                }
                let side_line = if new {
                    new_lines.len()
                } else {
                    old_lines.len()
                };
                result.rows.push(Row {
                    kind,
                    old: old.then_some(o),
                    new: new.then_some(n),
                    text: text.into(),
                    side_line,
                });
                if old {
                    old_lines.push(text);
                    o += 1;
                    used_o += 1;
                }
                if new {
                    new_lines.push(text);
                    n += 1;
                    used_n += 1;
                }
                if used_o > oc || used_n > nc {
                    return Err(invalid("patch hunk count mismatch"));
                }
            }
            previous = Some((o, n));
        }
        if git_files.len() > 1 {
            return Err(invalid(format!(
                "multi-file patches are unsupported: {}",
                git_files.join(", ")
            )));
        }
        if files.len() > 1 {
            return Err(invalid(format!(
                "multi-file patches are unsupported: {}",
                files.join(", ")
            )));
        }
        if result.rows.is_empty() {
            return Err(invalid(
                "patch must contain text hunks (binary, rename-only and mode-only patches are unsupported)",
            ));
        }
        result.old = old_lines.join("\n");
        result.new = new_lines.join("\n");
        if !old_lines.is_empty() && !old_no_newline {
            result.old.push('\n');
        }
        if !new_lines.is_empty() && !new_no_newline {
            result.new.push('\n');
        }
        result.patch_source = Some(patch);
        Ok(result)
    }
    pub(crate) fn display(&self) -> Result<(Vec<Row>, String), Error> {
        if let Some(e) = &self.error {
            return Err(invalid(e));
        }
        let mut rows = self.rows.clone();
        if self.marked {
            let lines: Vec<_> = self.new.split('\n').collect();
            if self.first == 0 || self.first.checked_add(lines.len()).is_none() {
                return Err(invalid(
                    "first line must be positive and fit the line range",
                ));
            }
            for r in self.added.iter().chain(&self.removed) {
                if r.0.is_empty() || *r.0.start() == 0 || *r.0.end() > lines.len() {
                    return Err(invalid("diff marks are outside the source"));
                }
            }
            let (mut old, mut new) = (self.first, self.first);
            for (i, text) in lines.iter().enumerate() {
                let a = self.added.iter().any(|r| r.0.contains(&(i + 1)));
                let d = self.removed.iter().any(|r| r.0.contains(&(i + 1)));
                if a && d {
                    return Err(invalid("a line cannot be both added and removed"));
                }
                rows.push(Row {
                    kind: if a {
                        RowKind::Added
                    } else if d {
                        RowKind::Removed
                    } else {
                        RowKind::Context
                    },
                    old: (!a).then_some(old),
                    new: (!d).then_some(new),
                    text: (*text).into(),
                    side_line: i,
                });
                if !a {
                    old += 1;
                }
                if !d {
                    new += 1;
                }
            }
        } else if let Context::Lines(context) = self.context {
            let mut keep = vec![false; rows.len()];
            let mut covered = 0;
            for (i, row) in rows.iter().enumerate() {
                if matches!(row.kind, RowKind::Added | RowKind::Removed) {
                    let end = i.saturating_add(context).saturating_add(1).min(rows.len());
                    let start = i.saturating_sub(context).max(covered);
                    if start < end {
                        keep[start..end].fill(true);
                    }
                    covered = end;
                }
            }
            let mut folded = Vec::new();
            let mut hidden = 0;
            for (row, keep) in rows.into_iter().zip(keep) {
                if !keep && row.kind == RowKind::Context {
                    hidden += 1;
                    continue;
                }
                if hidden > 0 {
                    folded.push(Row {
                        kind: RowKind::Fold { hidden },
                        old: None,
                        new: None,
                        text: String::new(),
                        side_line: 0,
                    });
                    hidden = 0;
                }
                folded.push(row);
            }
            if hidden > 0 {
                folded.push(Row {
                    kind: RowKind::Fold { hidden },
                    old: None,
                    new: None,
                    text: String::new(),
                    side_line: 0,
                });
            }
            rows = folded;
        }
        let source = rows
            .iter()
            .filter(|r| !matches!(r.kind, RowKind::Fold { .. }))
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if source.len() > MAX_SOURCE_BYTES {
            return Err(InvalidOption::SourceTooLarge { len: source.len() }.into());
        }
        Ok((rows, source))
    }
    pub(crate) fn highlight(
        &self,
        renderer: &Renderer,
        language: Language,
        theme: &str,
        comments: bool,
    ) -> anyhow::Result<syntax::StyledSource> {
        let old = syntax::highlight(renderer, language, &self.old, theme, comments)?;
        let new = syntax::highlight(renderer, language, &self.new, theme, comments)?;
        let offsets = |s: &str| {
            let mut pos = 0;
            s.split('\n')
                .map(|line| {
                    let start = pos;
                    pos += line.len() + 1;
                    start
                })
                .collect::<Vec<_>>()
        };
        let oo = offsets(&self.old);
        let no = offsets(&self.new);
        let mut runs = Vec::new();
        let mut offset = 0;
        for row in self.display()?.0 {
            if matches!(row.kind, RowKind::Fold { .. }) {
                continue;
            }
            let removed = row.kind == RowKind::Removed && !self.marked;
            let (styled, starts) = if removed { (&old, &oo) } else { (&new, &no) };
            let start = starts[row.side_line];
            let end = start + row.text.len();
            let first = styled.runs.partition_point(|r| r.range.end <= start);
            for run in &styled.runs[first..] {
                if run.range.start >= end {
                    break;
                }
                let a = run.range.start.max(start);
                let b = run.range.end.min(end);
                syntax::push_run(
                    &mut runs,
                    offset + a - start..offset + b - start,
                    run.style,
                    run.comment,
                );
            }
            offset += row.text.len() + 1;
        }
        Ok(syntax::StyledSource {
            runs,
            background: new.background,
        })
    }
    pub(crate) fn words(&self) -> Result<Vec<Range<usize>>, Error> {
        if self.marked {
            return Ok(vec![]);
        }
        let rows = self.display()?.0;
        let mut ranges = Vec::new();
        let mut starts = Vec::new();
        let mut pos = 0;
        for r in &rows {
            starts.push(pos);
            if !matches!(r.kind, RowKind::Fold { .. }) {
                pos += r.text.len() + 1;
            }
        }
        let mut i = 0;
        while i < rows.len() {
            if rows[i].kind != RowKind::Removed {
                i += 1;
                continue;
            }
            let start = i;
            while i < rows.len() && rows[i].kind == RowKind::Removed {
                i += 1;
            }
            let added = i;
            while i < rows.len() && rows[i].kind == RowKind::Added {
                i += 1;
            }
            for (a, b) in (start..added).zip(added..i) {
                let aa: Vec<_> = rows[a].text.split_word_bounds().collect();
                let bb: Vec<_> = rows[b].text.split_word_bounds().collect();
                let d = TextDiff::configure()
                    .timeout(Duration::from_millis(50))
                    .diff_slices(&aa, &bb);
                if d.ratio() < 0.4 {
                    continue;
                }
                let (mut ao, mut bo) = (starts[a], starts[b]);
                for c in d.iter_all_changes() {
                    let len = c.value().len();
                    match c.tag() {
                        ChangeTag::Equal => {
                            ao += len;
                            bo += len;
                        }
                        ChangeTag::Delete => {
                            ranges.push(ao..ao + len);
                            ao += len;
                        }
                        ChangeTag::Insert => {
                            ranges.push(bo..bo + len);
                            bo += len;
                        }
                    }
                }
            }
        }
        ranges.sort_by_key(|r| r.start);
        Ok(ranges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gutter_width_uses_only_displayed_numbers() {
        let rows = Diff::from_patch("@@ -999 +8 @@\n-old\n+new\n")
            .unwrap()
            .rows()
            .unwrap();
        assert_eq!(DiffNumbers::New.largest(&rows), 8);
        assert_eq!(DiffNumbers::Old.largest(&rows), 999);
        assert_eq!(DiffNumbers::Both.largest(&rows), 999);
    }
    #[test]
    fn folds_and_empty_changes() {
        let d = Diff::compare("a\nb\nc\nd\ne\nf\ng", "a\nb\nc\nX\ne\nf\ng")
            .unwrap()
            .context(Context::Lines(1));
        let (rows, source) = d.display().unwrap();
        assert_eq!(source, "c\nd\nX\ne");
        assert_eq!(rows[0].kind, RowKind::Fold { hidden: 2 });
        assert_eq!(rows.last().unwrap().kind, RowKind::Fold { hidden: 2 });
        assert_eq!(
            Diff::compare("a\nb", "a\nb")
                .unwrap()
                .context(Context::All)
                .displayed_source(),
            "a\nb"
        );
    }
    #[test]
    fn words_and_split_highlight() {
        let d = Diff::compare("let value = 1;", "let value = 2;").unwrap();
        let source = d.displayed_source();
        let words = d.words().unwrap();
        assert_eq!(
            words.iter().map(|r| &source[r.clone()]).collect::<Vec<_>>(),
            ["1", "2"]
        );
        assert!(
            Diff::compare("aaaa", "zzzz")
                .unwrap()
                .words()
                .unwrap()
                .is_empty()
        );
        let r = Renderer::new();
        let d = Diff::compare("/* broken", "let value = 2;").unwrap();
        let actual = d
            .highlight(
                &r,
                "rust".parse().unwrap(),
                "original-monoprint-dark",
                false,
            )
            .unwrap();
        let expected = syntax::highlight(
            &r,
            "rust".parse().unwrap(),
            "let value = 2;",
            "original-monoprint-dark",
            false,
        )
        .unwrap();
        let offset = "/* broken\n".len();
        let mapped: Vec<_> = actual
            .runs
            .into_iter()
            .filter(|r| r.range.start >= offset)
            .map(|mut r| {
                r.range.start -= offset;
                r.range.end -= offset;
                r
            })
            .collect();
        assert_eq!(mapped, expected.runs);
    }
}

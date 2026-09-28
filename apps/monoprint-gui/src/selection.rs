//! Translate the editor's UTF-8 selections to the renderer's source coordinates.
use monoprint::TextRange;
use std::ops::Range;

pub fn lines(value: &str, count: usize) -> Vec<Range<usize>> {
    merge(
        crate::document::highlight_ranges(value, usize::MAX)
            .unwrap_or_default()
            .into_iter()
            .map(|(start, end)| (start - 1)..end.min(count))
            .collect(),
    )
}

pub fn format_lines(ranges: Vec<Range<usize>>) -> String {
    merge(ranges)
        .into_iter()
        .map(|range| {
            if range.len() == 1 {
                (range.start + 1).to_string()
            } else {
                format!("{}-{}", range.start + 1, range.end)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn selected_lines(source: &str, range: Range<usize>) -> Range<usize> {
    let positions = text_range(source, range);
    let end = if positions.end.1 == 1 && positions.end.0 > positions.start.0 {
        positions.end.0 - 1
    } else {
        positions.end.0
    };
    positions.start.0 - 1..end
}

pub fn text_range(source: &str, range: Range<usize>) -> TextRange {
    let position = |offset| {
        let prefix = source[..offset].replace("\r\n", "\n").replace('\r', "\n");
        (
            prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
            prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
        )
    };
    TextRange::new(position(range.start), position(range.end))
}

pub fn resolve(source: &str, value: &str) -> Option<Vec<Range<usize>>> {
    fn offset(source: &str, position: (usize, usize)) -> Option<usize> {
        let (mut row, mut column) = (1, 1);
        let mut chars = source.char_indices().peekable();
        while let Some((offset, character)) = chars.next() {
            if (row, column) == position {
                return Some(offset);
            }
            match character {
                '\r' | '\n' => {
                    if character == '\r' && chars.peek().is_some_and(|(_, c)| *c == '\n') {
                        chars.next();
                    }
                    row += 1;
                    column = 1;
                }
                _ => column += 1,
            }
        }
        ((row, column) == position).then_some(source.len())
    }
    let mut ranges = Vec::new();
    for value in value.split(',').filter(|value| !value.trim().is_empty()) {
        let range = value.parse::<TextRange>().ok()?;
        let start = offset(source, range.start)?;
        let end = offset(source, range.end)?;
        if start >= end {
            return None;
        }
        ranges.push(start..end);
    }
    Some(merge(ranges))
}

pub fn format(source: &str, ranges: Vec<Range<usize>>) -> String {
    merge(ranges)
        .into_iter()
        .map(|range| text_range(source, range).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn covers(ranges: &[Range<usize>], selection: &Range<usize>) -> bool {
    if selection.is_empty() {
        return false;
    }
    // Edits can bring originally separate decorations together. They remain ordered.
    let mut end = selection.start;
    for range in ranges {
        if range.start > end {
            return false;
        }
        end = end.max(range.end);
        if end >= selection.end {
            return true;
        }
    }
    false
}

pub fn containing(ranges: Vec<Range<usize>>, offset: usize) -> Option<Range<usize>> {
    merge(ranges)
        .into_iter()
        .find(|range| range.contains(&offset))
}

pub fn toggle(ranges: Vec<Range<usize>>, selection: Range<usize>) -> Vec<Range<usize>> {
    let mut ranges = merge(ranges);
    if !covers(&ranges, &selection) {
        ranges.push(selection);
        return merge(ranges);
    }
    ranges
        .into_iter()
        .flat_map(|range| {
            if range.end <= selection.start || range.start >= selection.end {
                vec![range]
            } else {
                [
                    range.start..selection.start.min(range.end),
                    selection.end.max(range.start)..range.end,
                ]
                .into_iter()
                .filter(|range| !range.is_empty())
                .collect()
            }
        })
        .collect()
}

fn merge(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.sort_unstable_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges.into_iter().filter(|range| !range.is_empty()) {
        if let Some(previous) = merged.last_mut()
            && previous.end >= range.start
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_coordinates_round_trip_unicode_tabs_and_line_endings() {
        let source = "é\t<\r\n// β\rend\n";
        let ranges = resolve(source, "1:2-2:4, 3:1-3:4").unwrap();
        assert_eq!(&source[ranges[0].clone()], "\t<\r\n// ");
        assert_eq!(&source[ranges[1].clone()], "end");
        assert_eq!(format(source, ranges), "1:2-2:4, 3:1-3:4");
        for value in ["1:1-1:1", "0:1-2:1", "1:2-9:1", "not a range"] {
            assert!(resolve(source, value).is_none());
        }
    }
    #[test]
    fn adding_merges_and_removing_preserves_unselected_text() {
        assert!(covers(&[1..4, 4..10], &(2..9)));
        assert!(!covers(&[1..4, 5..10], &(2..9)));
        assert_eq!(toggle(vec![1..4, 8..10], 3..8), vec![1..10]);
        assert_eq!(
            toggle(std::iter::once(1..10).collect(), 4..7),
            vec![1..4, 7..10]
        );
        assert!(toggle(std::iter::once(1..10).collect(), 1..10).is_empty());
    }
    #[test]
    fn line_toggles_split_ranges_and_bound_saved_marks_without_expansion() {
        let ranges = lines("1-3, 5-99999999", 6);
        assert_eq!(format_lines(ranges.clone()), "1-3, 5-6");
        assert_eq!(format_lines(toggle(ranges, 1..2)), "1, 3, 5-6");
        assert!(lines("20-99999999", 6).is_empty());
        assert_eq!(selected_lines("é\nnext", 0..3), 0..1);
    }
}

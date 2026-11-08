use super::{Origin, RenderError, RenderIssue, SourceSpan, TextOffset};
use std::fmt::Write;

pub(super) struct Buffer {
    pub text: String,
    pub spans: Vec<SourceSpan>,
    pub issues: Vec<RenderIssue>,
    pub depth: usize,
    /// A source attachment is comment-only; it still uses the one renderer and
    /// records origins while writing, rather than editing completed text.
    pub abi_comment: bool,
    pub origin_only: bool,
    pub outline: Option<super::Outline>,
    pub outline_method: Option<crate::MethodId>,
    pub outline_header: Option<Origin>,
    pub outline_kind: &'static str,
    line_start: usize,
    line_end: Option<usize>,
    hide_line: bool,
    allocation: Option<std::collections::TryReserveError>,
}
impl Buffer {
    pub fn new(outline: bool) -> Self {
        Self {
            text: String::new(),
            spans: Vec::new(),
            issues: Vec::new(),
            depth: 0,
            abi_comment: false,
            origin_only: false,
            outline: outline.then(super::Outline::new),
            outline_method: None,
            outline_header: None,
            outline_kind: "code",
            line_start: 0,
            line_end: None,
            hide_line: false,
            allocation: None,
        }
    }
    pub fn put(&mut self, text: &str) -> Result<(), RenderError> {
        if self.origin_only {
            return Ok(());
        }
        self.append(text)
    }
    fn append(&mut self, text: &str) -> Result<(), RenderError> {
        self.text
            .try_reserve(text.len())
            .map_err(RenderError::Allocation)?;
        if let Some(outline) = &mut self.outline {
            outline
                .lines
                .try_reserve(text.bytes().filter(|&b| b == b'\n').count())
                .map_err(RenderError::Allocation)?;
        }
        let start = self.text.len();
        self.text.push_str(text);
        if let Some(outline) = &mut self.outline {
            for (offset, byte) in text.bytes().enumerate() {
                if byte != b'\n' {
                    continue;
                }
                let end = start + offset;
                if !self.hide_line && end > self.line_start {
                    outline.lines.push(super::OutlineLine {
                        start: TextOffset(self.line_start),
                        end: TextOffset(self.line_end.unwrap_or(end)),
                        depth: self.depth,
                        method: self.outline_method,
                        header: self.outline_header,
                        kind: self.outline_kind,
                    });
                }
                self.line_start = end + 1;
                self.line_end = None;
                self.hide_line = false;
                self.outline_header = None;
                self.outline_kind = "code";
            }
        }
        Ok(())
    }
    pub fn fmt(&mut self, args: std::fmt::Arguments<'_>) -> Result<(), RenderError> {
        self.write_fmt(args)
            .map_err(|_| match self.allocation.take() {
                Some(e) => RenderError::Allocation(e),
                None => RenderError::OutputInvariant("formatting an existing display fact failed"),
            })
    }
    pub fn finish_outline(&mut self) -> Result<(), RenderError> {
        if let Some(outline) = &mut self.outline
            && !self.hide_line
            && self.text.len() > self.line_start
        {
            outline
                .lines
                .try_reserve(1)
                .map_err(RenderError::Allocation)?;
            outline.lines.push(super::OutlineLine {
                start: TextOffset(self.line_start),
                end: TextOffset(self.line_end.unwrap_or(self.text.len())),
                depth: self.depth,
                method: self.outline_method,
                header: self.outline_header,
                kind: self.outline_kind,
            });
        }
        Ok(())
    }
    pub fn mark_outline(&mut self, kind: &'static str) {
        if !self.origin_only {
            self.outline_kind = kind;
        }
    }
    pub fn position(&self) -> TextOffset {
        TextOffset(self.text.len())
    }
    pub fn span(&mut self, start: TextOffset, origin: Origin) -> Result<(), RenderError> {
        self.spans.try_reserve(1).map_err(RenderError::Allocation)?;
        self.spans.push(SourceSpan {
            start,
            end: self.position(),
            origin,
        });
        Ok(())
    }
    pub fn indent(&mut self) -> Result<(), RenderError> {
        if self.origin_only {
            return Ok(());
        }
        let n = self.depth.checked_mul(4).ok_or(RenderError::Capacity)?;
        self.text.try_reserve(n).map_err(RenderError::Allocation)?;
        for _ in 0..n {
            self.text.push(' ');
        }
        if self.abi_comment {
            self.put("// ABI: ")?;
        }
        Ok(())
    }
    pub fn line(&mut self, s: &str) -> Result<(), RenderError> {
        self.indent()?;
        self.put(s)?;
        self.put("\n")
    }
    pub fn open(&mut self) -> Result<(), RenderError> {
        self.put(" ")?;
        self.open_plain()
    }
    /// A block without a header line of its own, indented at the block start.
    pub fn open_plain(&mut self) -> Result<(), RenderError> {
        if !self.origin_only {
            self.line_end = Some(self.text.len());
        }
        self.put("{\n")?;
        self.enter()
    }
    /// Opens a block whose header states one trailing note on its `{` line,
    /// the way a statement's note follows its `;`.
    pub fn open_note(&mut self, note: &str) -> Result<(), RenderError> {
        self.put(" { ")?;
        self.put(note)?;
        self.put("\n")?;
        self.enter()
    }
    fn enter(&mut self) -> Result<(), RenderError> {
        self.depth = self.depth.checked_add(1).ok_or(RenderError::Capacity)?;
        Ok(())
    }
    pub fn close(&mut self) -> Result<(), RenderError> {
        if !self.origin_only {
            self.hide_line = true;
        }
        self.depth = self
            .depth
            .checked_sub(1)
            .ok_or(RenderError::OutputInvariant("unbalanced output scope"))?;
        self.line("}")
    }
    /// Closes a block whose body printed nothing as `{ }` on its header line.
    /// `start` is the text position before the header line's opening brace; the
    /// replaced text is only that brace line and its closing-brace indentation,
    /// so no printed body text is discarded. A zero-length anchor recorded at
    /// the empty body start still points into the rewritten `{ }`.
    pub fn close_empty(&mut self, start: TextOffset) -> Result<(), RenderError> {
        self.depth = self
            .depth
            .checked_sub(1)
            .ok_or(RenderError::OutputInvariant("unbalanced output scope"))?;
        self.text.truncate(start.0);
        if let Some(outline) = &mut self.outline {
            // The empty-body rewrite replaces the already recorded header line.
            // Its replacement keeps the same header identity and brace-free range.
            let begin = self.text.rfind('\n').map_or(0, |n| n + 1);
            if let Some(line) = outline.lines.last()
                && line.start.0 == begin
            {
                self.outline_header = line.header;
            }
            outline
                .lines
                .truncate(outline.lines.partition_point(|line| line.start.0 < begin));
            self.line_start = begin;
            self.line_end = Some(start.0);
        }
        self.put(" { }\n")
    }
    /// Exact UTF-16 identity is retained through escaping; comments and names
    /// cannot inject new source structure. Only valid scalars are encoded as UTF-8.
    pub fn quote(&mut self, s: &str) -> Result<(), RenderError> {
        self.encode(s.encode_utf16(), Some("\""))
    }
    pub fn units(&mut self, units: &[u16], quoted: bool) -> Result<(), RenderError> {
        self.encode(units.iter().copied(), quoted.then_some("\""))
    }
    fn encode(
        &mut self,
        units: impl IntoIterator<Item = u16>,
        delimiter: Option<&str>,
    ) -> Result<(), RenderError> {
        if let Some(delimiter) = delimiter {
            self.put(delimiter)?;
        }
        for c in char::decode_utf16(units) {
            match c {
                Err(e) => self.fmt(format_args!("\\u{:04x}", e.unpaired_surrogate()))?,
                Ok('"') if delimiter == Some("\"") => self.put("\\\"")?,
                Ok('`') if delimiter == Some("`") => self.put("\\`")?,
                Ok('\\') => self.put("\\\\")?,
                Ok('\n') if delimiter != Some("`") => self.put("\\n")?,
                Ok('\r') if delimiter != Some("`") => self.put("\\r")?,
                Ok('\t') if delimiter != Some("`") => self.put("\\t")?,
                Ok(c)
                    if c.is_control()
                        || matches!(c, '\u{2028}' | '\u{2029}'|'\u{061c}'|'\u{200e}'..='\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}') =>
                {
                    self.fmt(format_args!("\\u{:04x}", c as u32))?
                }
                Ok(c) => {
                    let mut bytes = [0u8; 4];
                    self.put(c.encode_utf8(&mut bytes))?;
                }
            }
        }
        if let Some(delimiter) = delimiter {
            self.put(delimiter)?;
        }
        Ok(())
    }
    pub fn kotlin_name(&mut self, units: &[u16]) -> Result<(), RenderError> {
        self.encode(
            units.iter().copied(),
            (!kotlin_plain(units.iter().copied())).then_some("`"),
        )
    }
    pub fn kotlin_name_str(&mut self, name: &str) -> Result<(), RenderError> {
        self.encode(
            name.encode_utf16(),
            (!kotlin_plain(name.encode_utf16())).then_some("`"),
        )
    }
    pub fn kotlin_path(&mut self, units: &[u16], separator: u16) -> Result<(), RenderError> {
        for (index, segment) in units.split(|&unit| unit == separator).enumerate() {
            if index > 0 {
                self.put(".")?;
            }
            self.kotlin_name(segment)?;
        }
        Ok(())
    }
    pub fn name(&mut self, units: &[u16], form: crate::p7::NameForm) -> Result<(), RenderError> {
        if form == crate::p7::NameForm::Original {
            self.encode(units.iter().copied(), Some("`"))
        } else {
            self.units(units, false)
        }
    }
    pub fn path(&mut self, units: &[u16], separator: u16) -> Result<(), RenderError> {
        for (index, segment) in units.split(|&unit| unit == separator).enumerate() {
            if index > 0 {
                self.put(".")?;
            }
            self.name(segment, crate::substrate::names::name_form(segment))?;
        }
        Ok(())
    }
    pub fn problem(
        &mut self,
        method: crate::MethodId,
        cause: super::LocalRenderError,
    ) -> Result<(), RenderError> {
        let message = match &cause {
            super::LocalRenderError::Invariant(s) => *s,
            super::LocalRenderError::Scratch(_) => "method rendering scratch allocation failed",
        };
        let kind = cause.kind();
        if let Err(allocation) = self.issues.try_reserve(1) {
            return Err(RenderError::DiagnosticRetention {
                method,
                cause,
                allocation,
            });
        }
        self.issues.push(RenderIssue {
            method: Some(method),
            detail: message,
            kind,
            cause,
        });
        self.indent()?;
        self.put("unrecovered_rendering ")?;
        self.quote(message)?;
        self.put(";\n")
    }
}
fn kotlin_plain(units: impl Iterator<Item = u16> + Clone) -> bool {
    let mut rest = units.clone();
    let letter = |unit: u16| unit == 95 || (65..=90).contains(&unit) || (97..=122).contains(&unit);
    if !rest.next().is_some_and(letter)
        || !rest.all(|unit| letter(unit) || (48..=57).contains(&unit))
    {
        return false;
    }
    ![
        "as",
        "break",
        "class",
        "continue",
        "do",
        "else",
        "false",
        "for",
        "fun",
        "if",
        "in",
        "interface",
        "is",
        "null",
        "object",
        "package",
        "return",
        "super",
        "this",
        "throw",
        "true",
        "try",
        "typealias",
        "typeof",
        "val",
        "var",
        "when",
        "while",
    ]
    .iter()
    .any(|keyword| units.clone().eq(keyword.encode_utf16()))
}
impl Write for Buffer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if self.origin_only {
            return Ok(());
        }
        self.append(s).map_err(|error| {
            if let RenderError::Allocation(error) = error {
                self.allocation = Some(error);
            }
            std::fmt::Error
        })
    }
}

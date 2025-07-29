//! Diagnostic handling for TableGen parsing errors and warnings.

use std::marker::PhantomData;

use crate::raw::{
    TableGenDiagKind, TableGenSMDiagnosticRef, TableGenSMDiagnosticVectorRef,
    tableGenSMDiagnosticGetColumnNo, tableGenSMDiagnosticGetFilename, tableGenSMDiagnosticGetKind,
    tableGenSMDiagnosticGetLineNo, tableGenSMDiagnosticGetMessage, tableGenSMDiagnosticVectorGet,
};

use crate::string_ref::StringRef;

/// Iterator over diagnostics collected during TableGen parsing.
pub struct DiagnosticIter<'a> {
    raw: TableGenSMDiagnosticVectorRef,
    index: usize,
    _reference: PhantomData<&'a TableGenSMDiagnosticVectorRef>,
}

impl<'a> DiagnosticIter<'a> {
    /// Creates a new diagnostic iterator from a raw diagnostic vector.
    pub(crate) unsafe fn from_raw_vector(ptr: TableGenSMDiagnosticVectorRef) -> Self {
        Self {
            raw: ptr,
            index: 0,
            _reference: PhantomData,
        }
    }
}

impl<'a> Iterator for DiagnosticIter<'a> {
    type Item = Diagnostic<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe { tableGenSMDiagnosticVectorGet(self.raw, self.index) };
        self.index += 1;
        if next.is_null() {
            None
        } else {
            unsafe { Some(Diagnostic::from_raw(next)) }
        }
    }
}

/// A diagnostic message generated during TableGen parsing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Diagnostic<'a> {
    raw: TableGenSMDiagnosticRef,
    _reference: PhantomData<&'a TableGenSMDiagnosticRef>,
}

impl<'a> Diagnostic<'a> {
    /// Creates a new diagnostic from a raw pointer.
    pub(crate) unsafe fn from_raw(ptr: TableGenSMDiagnosticRef) -> Self {
        Self {
            raw: ptr,
            _reference: PhantomData,
        }
    }

    /// Returns the kind of this diagnostic.
    pub fn kind(&self) -> DiagnosticKind {
        let kind = unsafe { tableGenSMDiagnosticGetKind(self.raw) };
        DiagnosticKind::from_raw(kind)
    }

    /// Returns the diagnostic message.
    pub fn message(&self) -> StringRef {
        unsafe { StringRef::from_raw(tableGenSMDiagnosticGetMessage(self.raw)) }
    }

    /// Returns the filename where the diagnostic was generated.
    pub fn filename(&self) -> StringRef {
        unsafe { StringRef::from_raw(tableGenSMDiagnosticGetFilename(self.raw)) }
    }

    /// Returns the line number where the diagnostic was generated (1-based).
    pub fn line(&self) -> i32 {
        unsafe { tableGenSMDiagnosticGetLineNo(self.raw) }
    }

    /// Returns the column number where the diagnostic was generated (1-based).
    pub fn column(&self) -> i32 {
        unsafe { tableGenSMDiagnosticGetColumnNo(self.raw) }
    }
}

/// The kind of diagnostic message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Error,
    Warning,
    Remark,
    Note,
    Unknown(TableGenDiagKind::Type),
}

impl DiagnosticKind {
    /// Converts a raw diagnostic kind to the enum representation.
    pub(crate) fn from_raw(kind: TableGenDiagKind::Type) -> Self {
        match kind {
            TableGenDiagKind::DK_ERROR => Self::Error,
            TableGenDiagKind::DK_WARNING => Self::Warning,
            TableGenDiagKind::DK_REMARK => Self::Remark,
            TableGenDiagKind::DK_NOTE => Self::Note,
            _ => Self::Unknown(kind),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{TableGenParser, diagnostic::DiagnosticKind};

    #[test]
    fn parse_error() {
        let res = TableGenParser::new()
            .add_source("class A; invalid_token; class B;")
            .unwrap()
            .parse()
            .expect_err("invalid tablegen");

        let rk = res.record_keeper;
        assert!(rk.class("A").is_ok());
        assert!(rk.class("B").is_err());

        let diags = res.diagnostics;
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind(), DiagnosticKind::Error);
        assert_eq!(diags[0].filename().as_str().unwrap(), "");
        assert_eq!(diags[0].line(), 1);
        assert_eq!(diags[0].column(), 9);
        assert_eq!(
            diags[0].message().as_str().unwrap(),
            "Unexpected token at top level",
        );
    }
}

//! Utility functions and types for TableGen operations.

use std::{
    ffi::c_void,
    fmt::{self, Formatter},
};

use crate::{
    error::TableGenError, raw::TableGenFilePos, raw::TableGenStringRef, string_ref::StringRef,
};

/// Represents a position in a TableGen source file.
#[derive(Debug)]
pub struct FilePos {
    raw: TableGenFilePos,
}

impl FilePos {
    /// Creates a new [`FilePos`] from a raw [`TableGenFilePos`].
    ///
    /// # Safety
    ///
    /// The raw object must be valid.
    pub unsafe fn from_raw(raw: TableGenFilePos) -> Self {
        Self { raw }
    }

    /// Returns the filepath as a string reference.
    pub fn filepath(&self) -> StringRef<'_> {
        unsafe { StringRef::from_raw(self.raw.filepath) }
    }

    /// Returns the character position within the file (0-based).
    pub fn pos(&self) -> u32 {
        self.raw.pos
    }
}

pub(crate) unsafe extern "C" fn print_callback(string: TableGenStringRef, data: *mut c_void) {
    let (formatter, result) = unsafe { &mut *(data as *mut (&mut Formatter, fmt::Result)) };

    if result.is_err() {
        return;
    }

    *result = (|| {
        write!(
            formatter,
            "{}",
            TryInto::<&str>::try_into(unsafe { StringRef::from_raw(string) })
                .map_err(|_| fmt::Error)?
        )
    })();
}

pub(crate) unsafe extern "C" fn print_string_callback(
    string: TableGenStringRef,
    data: *mut c_void,
) {
    let (writer, result) = unsafe { &mut *(data as *mut (String, Result<(), TableGenError>)) };

    if result.is_err() {
        return;
    }

    *result = (|| {
        unsafe {
            writer.push_str(StringRef::from_raw(string).as_str()?);
        }

        Ok(())
    })();
}

#[cfg(test)]
mod tests {
    use crate::{TableGenParser, error::SourceLoc};

    #[test]
    fn record() {
        let source = "class Foo { int x; }";
        let rk = TableGenParser::new()
            .add_source(source)
            .unwrap()
            .parse()
            .expect("valid tablegen")
            .record_keeper;

        // class Foo { int x; }
        //       ^
        let foo = rk.class("Foo").expect("class Foo exists");
        let loc = foo.file_position(&rk).expect("valid location");
        assert_eq!(loc.pos(), 6);

        // class Foo { int x; }
        //                 ^
        let foo_x = foo.value("x").expect("x exists");
        let loc = foo_x.file_position(&rk).expect("valid location");
        assert_eq!(loc.pos(), 16);
    }
}

use std::{
    ffi::c_void,
    fmt::{self, Formatter},
    mem::MaybeUninit,
};

use crate::{
    SourceInfo,
    error::{SourceLocation, TableGenError},
    raw::{TableGenFilePos, TableGenStringRef, tableGenConvertLoc},
    string_ref::StringRef,
};

#[derive(Debug)]
pub struct FilePos {
    raw: TableGenFilePos,
}

impl FilePos {
    pub unsafe fn from_raw(raw: TableGenFilePos) -> Self {
        Self { raw }
    }

    pub fn filepath(&self) -> StringRef {
        unsafe { StringRef::from_raw(self.raw.filepath) }
    }

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

pub fn convert_loc(info: SourceInfo, loc: &SourceLocation) -> Option<FilePos> {
    let parser = info.0;

    let mut file_pos = MaybeUninit::<TableGenFilePos>::uninit();
    let result = unsafe { tableGenConvertLoc(parser.raw, loc.raw, file_pos.as_mut_ptr()) };
    if result == 0 {
        return None;
    }

    let file_pos_raw = unsafe { file_pos.assume_init() };
    let file_pos = unsafe { FilePos::from_raw(file_pos_raw) };
    Some(file_pos)
}

#[cfg(test)]
mod tests {
    use crate::{TableGenParser, error::SourceLoc, util::convert_loc};

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
        let loc = convert_loc(rk.source_info(), &foo.source_location()).expect("valid location");
        assert_eq!(loc.pos(), 6);

        // class Foo { int x; }
        //                 ^
        let foo_x = foo.value("x").expect("x exists");
        let loc = convert_loc(rk.source_info(), &foo_x.source_location()).expect("valid location");
        assert_eq!(loc.pos(), 16);
    }
}

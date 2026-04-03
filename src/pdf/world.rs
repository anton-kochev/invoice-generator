use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};
use typst_kit::fonts::{FontSearcher, FontSlot};

const TEMPLATE: &str = include_str!("template/invoice.typ");

/// Minimal `typst::World` implementation that compiles an invoice from
/// an in-memory JSON data blob and the embedded Typst template.
pub struct InvoiceWorld {
    source: Source,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,
    data_json: Bytes,
    data_file_id: FileId,
}

impl InvoiceWorld {
    /// Build a new world from serialized JSON invoice data.
    pub fn new(json_data: Vec<u8>) -> Self {
        let fonts = FontSearcher::new().search();

        let source_content = format!(
            "#let data = json(\"data.json\")\n\n{}",
            TEMPLATE
        );

        let main_id = FileId::new(None, VirtualPath::new("main.typ"));
        let data_file_id = FileId::new(None, VirtualPath::new("data.json"));

        Self {
            source: Source::new(main_id, source_content),
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
            data_json: Bytes::new(json_data),
            data_file_id,
        }
    }
}

impl typst::World for InvoiceWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.data_file_id {
            Ok(self.data_json.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts[index].get()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

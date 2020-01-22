// https://docs.microsoft.com/en-us/typography/opentype/spec/loca

use core::ops::Range;

use crate::parser::{Stream, LazyArray};
use crate::{Font, GlyphId, Result};

impl<'a> Font<'a> {
    pub(crate) fn glyph_range(&self, glyph_id: GlyphId) -> Result<Option<Range<usize>>> {
        use crate::head::IndexToLocationFormat as Format;

        // Check for overflow.
        if self.number_of_glyphs() == core::u16::MAX {
            return Ok(None);
        }

        let glyph_id = glyph_id.0;
        if glyph_id == core::u16::MAX {
            return Ok(None);
        }

        let total = self.number_of_glyphs() + 1;

        // Glyph ID must be smaller than total number of values in a `loca` array.
        if glyph_id + 1 >= total {
            return Ok(None);
        }

        let format = try_ok!(self.index_to_location_format());
        let mut s = Stream::new(self.loca?);
        let range = match format {
            Format::Short => {
                let array: LazyArray<u16> = s.read_array(total)?;
                // 'The actual local offset divided by 2 is stored.'
                array.at(glyph_id) as usize * 2 .. array.at(glyph_id + 1) as usize * 2
            }
            Format::Long  => {
                let array: LazyArray<u32> = s.read_array(total)?;
                array.at(glyph_id) as usize .. array.at(glyph_id + 1) as usize
            }
        };

        // TODO: use Range::is_empty as soon as it became stable
        if range.start == range.end {
            // No outline.
            Ok(None)
        } else if range.start > range.end {
            // 'The offsets must be in ascending order.'
            Ok(None)
        } else {
            Ok(Some(range))
        }
    }
}

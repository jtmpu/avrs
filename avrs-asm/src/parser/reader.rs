use std::io::Read;

use miette::SourceSpan;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ReadError {
    #[error("unexpected IO-error: {error}")]
    IO {
        error: std::io::Error,
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
    #[error("unexpected end of file")]
    UnexpectedEOF {
        #[label("error occured while reading here")]
        span: SourceSpan,
    },
}

pub struct Reader<T: Read, const SIZE: usize = 256> {
    reader: T,
    stream_offset: usize,
    buffer_offset: usize,
    buffer_size: usize,
    /// buffer_of is internal, to avoid unecessary filling of buffer
    buffer_eof: bool,
    /// EOF is used to track EOF via self.read(), for proper offset-count
    eof: bool,
    buffer: [u8; SIZE],
}

impl<T: Read, const SIZE: usize> Reader<T, SIZE> {
    pub fn new(reader: T) -> Self {
        Self {
            reader,
            stream_offset: 0,
            buffer_offset: 0,
            buffer_size: 0,
            buffer_eof: false,
            eof: false,
            buffer: [0u8; SIZE],
        }
    }

    fn buffer_empty(&self) -> bool {
        self.buffer_offset >= self.buffer_size
    }

    fn fill(&mut self) -> Result<(), ReadError> {
        if self.buffer_eof {
            return Ok(());
        }

        self.buffer_size = self.reader.read(&mut self.buffer)
            .map_err(|error| ReadError::IO{ error, span: (self.stream_offset, 0).into() })?;
        self.buffer_offset = 0;
        if self.buffer_size == 0 {
            self.buffer_eof = true;
        }

        Ok(())
    }

    pub fn peek(&mut self) -> Result<Option<u8>, ReadError> {
        if self.buffer_empty() {
            self.fill()?;
        }

        if self.buffer_eof {
            return Ok(None);
        }

        Ok(Some(self.buffer[self.buffer_offset]))
    }

    pub fn read(&mut self) -> Result<Option<u8>, ReadError> {
        if self.eof {
            return Ok(None);
        }

        let value = self.peek()?;
        self.buffer_offset += 1;
        self.stream_offset += 1;
        if value.is_none() {
            self.eof = true;
        }

        Ok(value)
    }

    pub fn read_exact(&mut self) -> Result<u8, ReadError> {
        if let Some(byte) = self.read()? {
            Ok(byte)
        } else {
            Err(ReadError::UnexpectedEOF { span: (self.stream_offset, 0).into() })
        }
    }

    pub fn offset(&self) -> usize {
        self.stream_offset
    }

    pub fn skip_until<P: Fn(u8) -> bool>(&mut self, predicate: P) -> Result<(), ReadError> {
        while let Some(byte) = self.peek()? {
            if predicate(byte) {
                break;
            }
            self.read()?;
        }
        Ok(())
    }
}

pub fn whitespace(byte: u8) -> bool {
    byte == b' ' || byte == b'\t' 
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create(value: &str) -> Reader<&[u8]> {
        Reader::new(value.as_bytes())
    }

    #[test]
    fn test_read() {
        let mut reader = create("abcd");
        assert_eq!(Some(b'a'), reader.read().unwrap());
        assert_eq!(1, reader.offset());
        assert_eq!(Some(b'b'), reader.read().unwrap());
        assert_eq!(2, reader.offset());
        assert_eq!(Some(b'c'), reader.read().unwrap());
        assert_eq!(3, reader.offset());
        assert_eq!(Some(b'd'), reader.read().unwrap());
        assert_eq!(4, reader.offset());
        assert_eq!(None, reader.read().unwrap());
        assert_eq!(5, reader.offset());
        assert_eq!(None, reader.read().unwrap());
        assert_eq!(5, reader.offset());
    }

    #[test]
    fn test_peek() {
        let mut reader = create("abc");
        assert_eq!(Some(b'a'), reader.peek().unwrap());
        assert_eq!(Some(b'a'), reader.peek().unwrap());
        assert_eq!(Some(b'a'), reader.peek().unwrap());
        assert_eq!(Some(b'a'), reader.read().unwrap());
        assert_eq!(1, reader.offset());
        assert_eq!(Some(b'b'), reader.peek().unwrap());
        assert_eq!(Some(b'b'), reader.peek().unwrap());
        assert_eq!(Some(b'b'), reader.read().unwrap());
        assert_eq!(2, reader.offset());
        assert_eq!(Some(b'c'), reader.peek().unwrap());
        assert_eq!(Some(b'c'), reader.peek().unwrap());
        assert_eq!(Some(b'c'), reader.read().unwrap());
        assert_eq!(3, reader.offset());
        assert_eq!(None, reader.peek().unwrap());
        assert_eq!(None, reader.peek().unwrap());
        assert_eq!(None, reader.read().unwrap());
        assert_eq!(4, reader.offset());
    }

    #[test]
    fn test_skip() {
        let mut reader = create("    abc");
        reader.skip_until(|b| !whitespace(b)).unwrap();

        assert_eq!(Some(b'a'), reader.read().unwrap());
        assert_eq!(5, reader.offset());
        assert_eq!(Some(b'b'), reader.read().unwrap());
        assert_eq!(6, reader.offset());
        assert_eq!(Some(b'c'), reader.peek().unwrap());
        assert_eq!(Some(b'c'), reader.peek().unwrap());
        assert_eq!(Some(b'c'), reader.read().unwrap());
        assert_eq!(7, reader.offset());
        assert_eq!(None, reader.read().unwrap());
        assert_eq!(8, reader.offset());
    }

    #[test]
    fn test_buffer_boundaries_read() {
        let value = "hello";
        let mut reader = Reader::<&[u8], 2>::new(value.as_bytes());

        let mut buffer = vec![];
        while let Some(byte) = reader.read().unwrap() {
            buffer.push(byte);
        }
        assert_eq!(str::from_utf8(&buffer).unwrap(), value);
    }

    #[test]
    fn test_buffer_boundaries_peek() {
        let value = "hello";
        let mut reader = Reader::<&[u8], 2>::new(value.as_bytes());

        let mut buffer = vec![];

        reader.peek().unwrap();
        buffer.push(reader.read().unwrap().unwrap());
        buffer.push(reader.read().unwrap().unwrap());
        // boundary hit
        reader.peek().unwrap();
        buffer.push(reader.read().unwrap().unwrap());
        buffer.push(reader.read().unwrap().unwrap());
        buffer.push(reader.read().unwrap().unwrap());

        assert_eq!(str::from_utf8(&buffer).unwrap(), value);
    }
}

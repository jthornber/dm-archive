use anyhow::Result;
use io::prelude::*;
use std::fs::File;
use std::io;
use std::ops::Range;
use std::os::unix::fs::FileExt;

use crate::run_iter::*;

//-----------------------------------------

#[derive(Debug)]
pub enum Chunk {
    Mapped(Vec<u8>),
    Unmapped(u64),
}

pub struct ThickChunker {
    input: File,
    input_size: u64,
    total_read: u64,
    block_size: usize,
}

impl ThickChunker {
    pub fn new(input: File, block_size: usize) -> Result<Self> {
        let input_size = input.metadata()?.len();

        Ok(Self {
            input,
            input_size,
            total_read: 0,
            block_size,
        })
    }

    // FIXME: stop reallocating and zeroing these buffers
    fn do_read(&mut self, mut buffer: Vec<u8>) -> Result<Option<Chunk>> {
        self.input.read_exact(&mut buffer)?;
        self.total_read += buffer.len() as u64;
        Ok(Some(Chunk::Mapped(buffer)))
    }

    fn next_chunk(&mut self) -> Result<Option<Chunk>> {
        let remaining = self.input_size - self.total_read;

        if remaining == 0 {
            Ok(None)
        } else if remaining >= self.block_size as u64 {
            let buf = vec![0; self.block_size];
            self.do_read(buf)
        } else {
            let buf = vec![0; remaining as usize];
            self.do_read(buf)
        }
    }
}

impl Iterator for ThickChunker {
    type Item = Result<Chunk>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_chunk() {
            Err(e) => Some(Err(e)),
            Ok(None) => None,
            Ok(Some(c)) => Some(Ok(c)),
        }
    }
}

//-----------------------------------------

pub struct ThinChunker {
    input: File,
    run_iter: RunIter,
    data_block_size: u64,

    max_read_size: usize,
    current_run: Option<(bool, Range<u64>)>,
}

impl ThinChunker {
    pub fn new(input: File, run_iter: RunIter, data_block_size: u64) -> Self {
        Self {
            input,
            run_iter,
            data_block_size,

            max_read_size: 16 * 1024 * 1024,
            current_run: None,
        }
    }

    fn next_run_bytes(&mut self) -> Option<(bool, Range<u64>)> {
        self.run_iter.next().map(|(b, Range { start, end })| {
            (
                b,
                Range {
                    start: start as u64 * self.data_block_size,
                    end: end as u64 * self.data_block_size,
                },
            )
        })
    }

    fn next_chunk(&mut self) -> Result<Option<Chunk>> {
        let mut run = None;
        std::mem::swap(&mut run, &mut self.current_run);

        match run.or_else(|| self.next_run_bytes()) {
            Some((false, run)) => Ok(Some(Chunk::Unmapped(run.end - run.start))),
            Some((true, run)) => {
                let run_len = run.end - run.start;
                if run_len <= self.max_read_size as u64 {
                    let mut buf = vec![0; run_len as usize];
                    self.input.read_exact_at(&mut buf, run.start)?;
                    Ok(Some(Chunk::Mapped(buf)))
                } else {
                    let mut buf = vec![0; self.max_read_size];
                    self.input.read_exact_at(&mut buf, run.start)?;
                    self.current_run = Some((true, (run.start + buf.len() as u64)..run.end));
                    Ok(Some(Chunk::Mapped(buf)))
                }
            }
            None => Ok(None),
        }
    }
}

impl Iterator for ThinChunker {
    type Item = Result<Chunk>;

    fn next(&mut self) -> Option<Self::Item> {
        let mc = self.next_chunk();
        match mc {
            Err(e) => Some(Err(e)),
            Ok(Some(c)) => Some(Ok(c)),
            Ok(None) => None,
        }
    }
}

//-----------------------------------------

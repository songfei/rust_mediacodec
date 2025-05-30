use std::{
    ffi::{CStr, CString},
    marker::PhantomData,
    os::raw::c_char,
};

use log::{debug, info};

use crate::{AMediaFormat, CodecInputBuffer, MediaFormat, MediaStatus};

pub enum SeekMode {
    SeekClosestSync,
    SeekNextSync,
    SeekPreviousSync
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AMediaExtractor {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

#[link(name = "mediandk")]
extern "C" {
    /// Since: API 21
    fn AMediaExtractor_new() -> *mut AMediaExtractor;

    /// Since: API 21
    fn AMediaExtractor_delete(extractor: *mut AMediaExtractor) -> isize;

    /// Since: API 21
    fn AMediaExtractor_setDataSourceFd(
        extractor: *mut AMediaExtractor,
        fd: i32,
        offset: u64,
        length: u64,
    ) -> isize;

    /// Since: API 21
    fn AMediaExtractor_setDataSource(
        extractor: *mut AMediaExtractor,
        location: *const c_char,
    ) -> isize;

    /// Since: API 21
    fn AMediaExtractor_getTrackCount(extractor: *mut AMediaExtractor) -> usize;

    /// Since: API 21
    fn AMediaExtractor_getTrackFormat(
        extractor: *mut AMediaExtractor,
        index: usize,
    ) -> *mut AMediaFormat;

    /// Since: API 21
    fn AMediaExtractor_selectTrack(extractor: *mut AMediaExtractor, index: usize) -> isize;

    /// Since: API 21
    fn AMediaExtractor_unselectTrack(extractor: *mut AMediaExtractor, index: usize) -> isize;

    /// Since: API 21
    fn AMediaExtractor_readSampleData(
        extractor: *mut AMediaExtractor,
        buffer: *mut u8,
        capacity: usize,
    ) -> isize;

    /// Since: API 21
    fn AMediaExtractor_getSampleFlags(extractor: *mut AMediaExtractor) -> u32;

    /// Since: API 21
    fn AMediaExtractor_getSampleTrackIndex(extractor: *mut AMediaExtractor) -> i32;

    /// Since: API 21
    fn AMediaExtractor_getSampleTime(extractor: *mut AMediaExtractor) -> i64;

    /// Since: API 21
    fn AMediaExtractor_advance(extractor: *mut AMediaExtractor) -> bool;

    /// Since: API 21
    fn AMediaExtractor_seekTo(extractor: *mut AMediaExtractor, seek_pos_us: i64, mode: i32) -> i32;
}

/// MediaExtractor is a demuxer that opens a file or resource and demuxes the data to hand over to MediaCodec
#[derive(Debug)]
pub struct MediaExtractor {
    inner: *mut AMediaExtractor,
    has_next: bool,
}

impl MediaExtractor {
    /// Creates a new MediaExtractor
    fn new() -> Self {
        Self {
            inner: unsafe { AMediaExtractor_new() },
            has_next: false,
        }
    }

    /// Creates a MediaExtractor with data source set to a specific URL
    pub fn from_url(path: &str) -> Result<Self, MediaStatus> {
        unsafe {
            let mut me = Self::new();

            let path = CString::new(path).unwrap();

            let result = AMediaExtractor_setDataSource(me.inner, path.as_ptr());
            MediaStatus::make_result(result)?;

            me.has_next = true;

            Ok(me)
        }
    }

    /// Returns the number of tracks found by MediaExtractor
    pub fn track_count(&self) -> usize {
        unsafe { AMediaExtractor_getTrackCount(self.inner) }
    }

    /// Returns the track index of the current packet to be retrieved by MediaExtractor
    pub fn track_index(&self) -> i32 {
        unsafe { AMediaExtractor_getSampleTrackIndex(self.inner) }
    }

    /// Returns the MediaFormat containing the parameters for this track index.
    ///
    /// The format can be used to create and initialize MediaCodec
    pub fn track_format(&self, index: usize) -> Option<MediaFormat> {
        unsafe {
            if self.track_count() <= index {
                debug!("Invalid track index {index}");
                return None;
            }

            let fmt = AMediaExtractor_getTrackFormat(self.inner, index);
            Some(MediaFormat::from_raw(fmt))
        }
    }

    /// Select this track to be demuxed by MediaExtractor
    pub fn select_track(&mut self, index: usize) {
        unsafe {
            AMediaExtractor_selectTrack(self.inner, index);
        }
    }

    /// Unselect this track to be demuxed by MediaExtractor
    pub fn unselect_track(&mut self, index: usize) {
        unsafe {
            AMediaExtractor_unselectTrack(self.inner, index);
        }
    }

    /// Returns the sample flags for the current packet to be returned
    pub fn sample_flags(&self) -> u32 {
        unsafe { AMediaExtractor_getSampleFlags(self.inner) }
    }

    /// Returns the time for the current packet to be returned
    pub fn sample_time(&self) -> i64 {
        unsafe { AMediaExtractor_getSampleTime(self.inner) }
    }

    /// Read a packet into `buffer` and advance the extractor
    /// Returns true if there's still more data to read
    pub fn read_next(&mut self, buffer: &mut CodecInputBuffer) -> bool {
        unsafe {
            if !self.has_next {
                return false;
            }

            let count = AMediaExtractor_readSampleData(self.inner, buffer.buffer, buffer.size);
            if count > 0 {
                buffer.set_write_size(count as usize);
                buffer.set_time(self.sample_time() as u64);
            }
            buffer.set_flags(self.sample_flags());

            self.has_next = AMediaExtractor_advance(self.inner);

            self.has_next
        }
    }

    /// Returns whether MediaExtractor still has packets to read
    pub fn has_next(&self) -> bool {
        self.has_next
    }
    
    pub fn seek_to(&self, pos_us: i64, mode: SeekMode) {
        unsafe {
            AMediaExtractor_seekTo(self.inner, pos_us, mode as i32);
        }
    }
}

impl Drop for MediaExtractor {
    fn drop(&mut self) {
        unsafe {
            info!("Deleting the extractor...");
            AMediaExtractor_delete(self.inner);
        }
    }
}

unsafe impl Send for MediaExtractor {}
unsafe impl Sync for MediaExtractor {}

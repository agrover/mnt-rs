// Copyright (C) 2014-2015 Mickaël Salaün
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation, version 3 of the License.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub struct ParseError {
    desc: String,
    // TODO: cause: Option<&'a (Error + 'a)>,
}

impl ParseError {
    pub fn new(detail: String) -> ParseError {
        ParseError {
            desc: format!("Mount parsing: {}", detail),
        }
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        self.desc.as_ref()
    }
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> ParseError {
        ParseError::new(format!("Failed to read the mounts file: {}", err))
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}", self.description())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineError {
    MissingSpec,
    MissingFile,
    InvalidFilePath(String),
    InvalidFile(String),
    MissingVfstype,
    MissingMntops,
    MissingFreq,
    InvalidFreq(String),
    MissingPassno,
    InvalidPassno(String),
    MissingId,
    InvalidId(String),
    MissingParentId,
    InvalidParentId(String),
    MissingMajMin,
    InvalidMajMin(String),
    MissingRoot,
    InvalidRoot(String),
    MissingOptional,
    InvalidOptional(String),
    MissingSuperOptions,
    InvalidSuperOptions(String),
}

impl fmt::Display for LineError {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let desc: Cow<_> = match *self {
            LineError::MissingSpec => "Missing field: spec".into(),
            LineError::MissingFile => "Missing field: file".into(),
            LineError::InvalidFilePath(ref f) => format!("Bad 'file' field value (not absolute path): {}", f).into(),
            LineError::InvalidFile(ref f) => format!("Bad 'file' field value: {}", f).into(),
            LineError::MissingVfstype => "Missing field: vfstype".into(),
            LineError::MissingMntops => "Missing field: mntops".into(),
            LineError::MissingFreq => "Missing field: freq".into(),
            LineError::InvalidFreq(ref f) => format!("Bad 'dump' field value: {}", f).into(),
            LineError::MissingPassno => "Missing field: passno".into(),
            LineError::InvalidPassno(ref f) => format!("Bad 'passno' field value: {}", f).into(),
            LineError::MissingId => "Missing field: id".into(),
            LineError::InvalidId(ref f) => format!("Bad 'id' field value: {}", f).into(),
            LineError::MissingParentId => "Missing field: parent id".into(),
            LineError::InvalidParentId(ref f) => format!("Bad 'parent id' field value: {}", f).into(),
            LineError::MissingMajMin => "Missing field: maj:min".into(),
            LineError::InvalidMajMin(ref f) => format!("Bad 'maj:min' field value: {}", f).into(),
            LineError::MissingRoot => "Missing field: root".into(),
            LineError::InvalidRoot(ref f) => format!("Bad 'root' field value: {}", f).into(),
            LineError::MissingOptional => "Missing field: optional".into(),
            LineError::InvalidOptional(ref f) => format!("Bad 'optional' field value: {}", f).into(),
            LineError::MissingSuperOptions => "Missing field: superoptions".into(),
            LineError::InvalidSuperOptions(ref f) => format!("Bad 'superoptions' field value: {}", f).into(),
        };
        write!(out, "Line parsing: {}", desc)
    }
}

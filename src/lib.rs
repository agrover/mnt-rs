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

pub use error::*;

mod error;
pub mod mount;

use std::str::FromStr;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MntOps {
    Atime(bool),
    DirAtime(bool),
    RelAtime(bool),
    Dev(bool),
    Exec(bool),
    Suid(bool),
    Write(bool),
    Extra(String),
}

impl FromStr for MntOps {
    type Err = LineError;

    fn from_str(token: &str) -> Result<MntOps, LineError> {
        Ok(match token {
               "atime" => MntOps::Atime(true),
               "noatime" => MntOps::Atime(false),
               "diratime" => MntOps::DirAtime(true),
               "nodiratime" => MntOps::DirAtime(false),
               "relatime" => MntOps::RelAtime(true),
               "norelatime" => MntOps::RelAtime(false),
               "dev" => MntOps::Dev(true),
               "nodev" => MntOps::Dev(false),
               "exec" => MntOps::Exec(true),
               "noexec" => MntOps::Exec(false),
               "suid" => MntOps::Suid(true),
               "nosuid" => MntOps::Suid(false),
               "rw" => MntOps::Write(true),
               "ro" => MntOps::Write(false),
               // TODO: Replace with &str
               extra => MntOps::Extra(extra.to_string()),
           })
    }
}

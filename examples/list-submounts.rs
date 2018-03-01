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

extern crate mnt;

use mnt::mount::{get_submounts, VecMountEntry};
use std::env::args;
use std::path::{Path, PathBuf};


fn list_submounts(root: &Path) {
    match get_submounts(&root) {
        Ok(list) => {
            for mount in list.remove_overlaps(&Vec::<&Path>::new()).iter() {
                println!("* {:?}", mount);
            }
        },
        Err(e) => println!("Error: {}", e),
    }
}

fn main() {
    let root = match args().skip(1).next() {
        Some(root) => PathBuf::from(root),
        None => PathBuf::from("/"),
    };
    list_submounts(&root);
}

// Copyright (C) 2014-2015 Mickaël Salaün
// Copyright (C) 2018 Andy Grover
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

// Support for parsing /proc/<pid>/mountinfo. Fields are based on description
// in the kernel's Documentation/filesystems/proc.txt section 3.5.

use error::*;
use std::collections::{HashMap, HashSet};
use std::convert::{AsRef, From};
use std::fs::File;
use std::io::{BufReader, BufRead, Lines};
use std::iter::Enumerate;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::MntOps;

const PROC_MOUNTINFO: &str = "/proc/self/mountinfo";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountInfoEntry {
    pub id: i32,
    pub parent_id: i32,
    pub major: u32,
    pub minor: u32,
    pub root: PathBuf,
    pub file: PathBuf,
    pub mntops: Vec<MntOps>,
    pub optionals: HashMap<String, Option<String>>,
    pub vfstype: String,
    pub spec: Option<String>,
    pub super_options: HashSet<String>,
}

#[derive(Clone, Debug)]
pub enum MountInfoParam<'a> {
    MountId(i32),
    ParentId(i32),
    Major(u32),
    Minor(u32),
    Root(&'a Path),
    MountPoint(&'a Path),
    MntOps(&'a MntOps),
    Optionals(&'a str),
    VfsType(&'a str),
    Spec(Option<&'a str>),
    SuperOptions(&'a str),
}

impl MountInfoEntry {
    pub fn contains(&self, search: &MountInfoParam) -> bool {
        match search {
            &MountInfoParam::MountId(id) => id == self.id,
            &MountInfoParam::ParentId(id) => id == self.parent_id,
            &MountInfoParam::Major(maj) => maj == self.major,
            &MountInfoParam::Minor(min) => min == self.minor,
            &MountInfoParam::Root(root) => root == self.root,
            &MountInfoParam::MountPoint(file) => file == &self.file,
            &MountInfoParam::MntOps(mntops) => self.mntops.contains(mntops),
            &MountInfoParam::Optionals(optional) => self.optionals.contains_key(optional),
            &MountInfoParam::VfsType(vfstype) => vfstype == &self.vfstype,
            &MountInfoParam::Spec(spec) => spec == self.spec.as_ref().map(|x| &**x),
            &MountInfoParam::SuperOptions(superops) => self.super_options.contains(superops),
        }
    }
}

impl FromStr for MountInfoEntry {
    type Err = LineError;

    fn from_str(line: &str) -> Result<MountInfoEntry, LineError> {
        let line = line.trim();
        let mut tokens = line.split_terminator(|s: char| s == ' ' || s == '\t')
            .filter(|s| s != &"");

        let id = try!(tokens.next().ok_or(LineError::MissingId)).parse().unwrap();
        let parent_id = try!(tokens.next().ok_or(LineError::MissingParentId))
            .parse()
            .unwrap();
        let (major, minor): (u32, u32) = {
            let majmin = try!(tokens.next().ok_or(LineError::MissingMajMin)).to_string();
            let mut spl = majmin.splitn(2, ":");
            let maj = spl.next().unwrap();
            let min = spl.next().unwrap();
            (maj.parse().unwrap(), min.parse().unwrap())
        };
        let root = PathBuf::from(try!(tokens.next().ok_or(LineError::MissingRoot)));
        let file = PathBuf::from(try!(tokens.next().ok_or(LineError::MissingFile)));
        let mntops =
            try!(tokens.next().ok_or(LineError::MissingMntops))
                // FIXME: Handle MntOps errors
                .split_terminator(',').map(|x| { FromStr::from_str(x).unwrap() }).collect();

        let mut optionals = HashMap::new();
        loop {
            let optional = try!(tokens.next().ok_or(LineError::MissingOptional)).to_string();
            if optional == "-" {
                break;
            }

            if optional.contains(":") {
                let mut spl = optional.splitn(2, ":");
                let tag = spl.next().unwrap();
                let value = spl.next().unwrap();
                optionals.insert(tag.to_owned(), Some(value.to_owned()));
            } else {
                optionals.insert(optional, None);
            }
        }

        let vfstype = try!(tokens.next().ok_or(LineError::MissingVfstype)).to_string();
        let spec = match try!(tokens.next().ok_or(LineError::MissingSpec)) {
            "none" => None,
            x => Some(x.to_owned()),
        };
        let super_options = try!(tokens.next().ok_or(LineError::MissingSuperOptions))
            .split_terminator(',')
            .map(|x| x.to_owned())
            .collect();

        Ok(MountInfoEntry {
               id,
               parent_id,
               major,
               minor,
               root,
               file,
               mntops,
               optionals,
               vfstype,
               spec,
               super_options,
           })
    }
}

/// Get a list of all mount points from `root` and beneath using a custom `BufRead`
pub fn get_submounts_from<T, U>(root: T, iter: MountInfoIter<U>) -> Result<Vec<MountInfoEntry>, ParseError>
    where T: AsRef<Path>,
          U: BufRead
{
    let mut ret = vec![];
    for mount in iter {
        match mount {
            Ok(m) => {
                if m.file.starts_with(&root) {
                    ret.push(m);
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(ret)
}

/// Get a list of all mount points from `root` and beneath using */proc/mounts*
pub fn get_submounts<T>(root: T) -> Result<Vec<MountInfoEntry>, ParseError>
    where T: AsRef<Path>
{
    get_submounts_from(root, try!(MountInfoIter::new_from_self()))
}

/// Get the mount point for the `target` using a custom `BufRead`
pub fn get_mount_from<T, U>(target: T, iter: MountInfoIter<U>) -> Result<Option<MountInfoEntry>, ParseError>
    where T: AsRef<Path>,
          U: BufRead
{
    let mut ret = None;
    for mount in iter {
        match mount {
            Ok(m) => {
                if target.as_ref().starts_with(&m.file) {
                    // Get the last entry
                    ret = Some(m);
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(ret)
}

/// Get the mount point for the `target` using */proc/mounts*
pub fn get_mount<T>(target: T) -> Result<Option<MountInfoEntry>, ParseError>
    where T: AsRef<Path>
{
    get_mount_from(target, try!(MountInfoIter::new_from_self()))
}

/// Find the potential mount point providing readable or writable access to a path
///
/// Do not check the path existence but its potentially parent mount point.
pub fn get_mount_writable<T>(target: T, writable: bool) -> Option<MountInfoEntry>
    where T: AsRef<Path>
{
    match get_mount(target) {
        Ok(Some(m)) => {
            if !writable || m.mntops.contains(&MntOps::Write(writable)) {
                Some(m)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub trait VecMountEntry {
    fn remove_overlaps<T>(self, exclude_files: &Vec<T>) -> Self where T: AsRef<Path>;
}

impl VecMountEntry for Vec<MountInfoEntry> {
    // FIXME: Doesn't work for moved mounts: they don't change order
    fn remove_overlaps<T>(self, exclude_files: &Vec<T>) -> Vec<MountInfoEntry>
        where T: AsRef<Path>
    {
        let mut sorted: Vec<MountInfoEntry> = vec![];
        let root = Path::new("/");
        'list: for mount in self.into_iter().rev() {
            // Strip fake root mounts (created from bind mounts)
            if AsRef::<Path>::as_ref(&mount.file) == root {
                continue 'list;
            }
            let mut has_overlaps = false;
            'filter: for mount_sorted in sorted.iter() {
                if exclude_files
                       .iter()
                       .skip_while(|x| {
                                       AsRef::<Path>::as_ref(&mount_sorted.file) != x.as_ref()
                                   })
                       .next()
                       .is_some() {
                    continue 'filter;
                }
                // Check for mount overlaps
                if mount.file.starts_with(&mount_sorted.file) {
                    has_overlaps = true;
                    break 'filter;
                }
            }
            if !has_overlaps {
                sorted.push(mount);
            }
        }
        sorted.reverse();
        sorted
    }
}

pub struct MountInfoIter<T: BufRead> {
    lines: Enumerate<Lines<T>>,
}

impl<T> MountInfoIter<T>
    where T: BufRead
{
    pub fn new(mtab: T) -> MountInfoIter<T> {
        MountInfoIter { lines: mtab.lines().enumerate() }
    }
}

impl MountInfoIter<BufReader<File>> {
    pub fn new_from_self() -> Result<MountInfoIter<BufReader<File>>, ParseError> {
        let file = try!(File::open(PROC_MOUNTINFO));
        Ok(MountInfoIter::new(BufReader::new(file)))
    }

    pub fn new_from_pid(pid: u32) -> Result<MountInfoIter<BufReader<File>>, ParseError> {
        let p: PathBuf = vec!["/proc/", &pid.to_string(), "/mountinfo"].iter().collect();
        let file = try!(File::open(p));
        Ok(MountInfoIter::new(BufReader::new(file)))
    }
}

impl<T> Iterator for MountInfoIter<T>
    where T: BufRead
{
    type Item = Result<MountInfoEntry, ParseError>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        match self.lines.next() {
            Some((nb, line)) => {
                Some(match line {
                         Ok(line) => {
                             match <MountInfoEntry as FromStr>::from_str(line.as_ref()) {
                                 Ok(m) => Ok(m),
                                 Err(e) => {
                                     Err(ParseError::new(format!("Failed at line {}: {}", nb, e)))
                                 }
                             }
                         }
                         Err(e) => Err(From::from(e)),
                     })
            }
            None => None,
        }
    }
}


#[cfg(test)]
mod test {
    use std::io::Cursor;
    use std::path::PathBuf;
    use super::{MntOps, MountInfoEntry, MountInfoIter, MountInfoParam};

    const TEST_MOUNTINFO: &str = "\
            20 66 0:20 / /sys rw,nosuid,nodev,noexec,relatime shared:2 - sysfs sysfs rw,seclabel
21 66 0:4 / /proc rw,nosuid,nodev,noexec,relatime shared:24 - proc proc rw
22 66 0:6 / /dev rw,nosuid shared:20 - devtmpfs devtmpfs rw,seclabel,size=7898068k,nr_inodes=1974517,mode=755
23 20 0:7 / /sys/kernel/security rw,nosuid,nodev,noexec,relatime shared:3 - securityfs securityfs rw
24 22 0:21 / /dev/shm rw,nosuid,nodev shared:21 - tmpfs tmpfs rw,seclabel
25 22 0:22 / /dev/pts rw,nosuid,noexec,relatime shared:22 - devpts devpts rw,seclabel,gid=5,mode=620,ptmxmode=000
26 66 0:23 / /run rw,nosuid,nodev shared:23 - tmpfs tmpfs rw,seclabel,mode=755
27 20 0:24 / /sys/fs/cgroup ro,nosuid,nodev,noexec shared:4 - tmpfs tmpfs ro,seclabel,mode=755
cgroup rw,seclabel,devices
39 27 0:36 / /sys/fs/cgroup/blkio rw,nosuid,nodev,noexec,relatime shared:15 - cgroup cgroup rw,seclabel,blkio
40 27 0:37 / /sys/fs/cgroup/cpu,cpuacct rw,nosuid,nodev,noexec,relatime shared:16 - cgroup cgroup rw,seclabel,cpu,cpuacct
63 20 0:38 / /sys/kernel/config rw,relatime shared:18 - configfs configfs rw
66 0 253:0 / / rw,relatime shared:1 - xfs /dev/mapper/luks-3334ad94-8d7e-4134-8ba3-a7677b2651ef rw,seclabel,attr2,inode64,noquota
41 20 0:19 / /sys/fs/selinux rw,relatime shared:19 - selinuxfs selinuxfs rw
42 21 0:40 / /proc/sys/fs/binfmt_misc rw,relatime shared:25 - autofs systemd-1 rw,fd=24,pgrp=1,timeout=0,minproto=5,maxproto=5,direct,pipe_ino=16858
43 20 0:8 / /sys/kernel/debug rw,relatime shared:26 - debugfs debugfs rw,seclabel
44 22 0:41 / /dev/hugepages rw,relatime shared:27 - hugetlbfs hugetlbfs rw,seclabel,pagesize=2M
45 22 0:18 / /dev/mqueue rw,relatime shared:28 - mqueue mqueue rw,seclabel
78 21 0:42 / /proc/fs/nfsd rw,relatime shared:29 - nfsd nfsd rw
80 66 0:43 / /tmp rw,nosuid,nodev shared:30 - tmpfs tmpfs rw,seclabel
82 66 8:1 / /boot rw,relatime shared:31 - ext4 /dev/sda1 rw,seclabel,data=ordered
84 66 0:44 / /var/lib/nfs/rpc_pipefs rw,relatime shared:32 - rpc_pipefs sunrpc rw
287 26 0:46 / /run/user/42 rw,nosuid,nodev,relatime shared:229 - tmpfs tmpfs rw,seclabel,size=1582224k,mode=700,uid=42,gid=42
433 26 0:48 / /run/user/1000 rw,nosuid,nodev,relatime shared:371 - tmpfs tmpfs rw,seclabel,size=1582224k,mode=700,uid=1001,gid=1001
444 433 0:49 / /run/user/1000/gvfs rw,nosuid,nodev,relatime shared:381 - fuse.gvfsd-fuse gvfsd-fuse rw,user_id=1001,group_id=1001
455 20 0:50 / /sys/fs/fuse/connections rw,relatime shared:391 - fusectl fusectl rw
493 26 179:1 / /run/media/agrover/A3D2-CF16 rw,nosuid,nodev,relatime shared:400 - vfat /dev/mmcblk0p1 rw,uid=1001,gid=1001,fmask=0022,dmask=0022,codepage=437,iocharset=ascii,shortname=mixed,showexec,utf8,flush,errors=remount-ro
        ";

    #[test]
    fn test_mountinfo_from() {
        use super::MntOps::*;
        use std::collections::{HashMap, HashSet};

        let buf = Cursor::new(TEST_MOUNTINFO);

        let mount_sysfs = MountInfoEntry {
            id: 20,
            parent_id: 66,
            major: 0,
            minor: 20,
            root: PathBuf::from("/"),
            file: PathBuf::from("/sys"),
            mntops: vec![Write(true),
                         Suid(false),
                         Dev(false),
                         Exec(false),
                         RelAtime(true)],
            optionals: {
                let mut m = HashMap::new();
                m.insert("shared".to_owned(), Some("2".to_owned()));
                m
            },
            vfstype: "sysfs".to_owned(),
            spec: Some("sysfs".to_owned()),
            super_options: {
                let mut s = HashSet::new();
                s.insert("rw".to_owned());
                s.insert("seclabel".to_owned());
                s
            },
        };

        // let mounts = MountInfoIter::new(buf.clone());
        // assert_eq!(mounts.map(|x| x.unwrap() ).collect::<Vec<_>>(), mounts_all.clone());
        // let mounts = MountIter::new(buf.clone());
        // assert_eq!(get_submounts_from("/", mounts).ok(), Some(mounts_all.clone()));
        // let mounts = MountIter::new(buf.clone());
        // assert_eq!(get_submounts_from("/var/tmp", mounts).ok(), Some(vec!(mount_vartmp.clone())));
        // let mounts = MountIter::new(buf.clone());
        // assert_eq!(get_mount_from("/var/tmp/bar", mounts).ok(), Some(Some(mount_vartmp.clone())));
        // let mounts = MountIter::new(buf.clone());
        // assert_eq!(get_mount_from("/var/", mounts).ok(), Some(Some(mount_root.clone())));

        // search
        // let mut mounts = MountInfoIter::new(buf.clone()).map(|m| m.ok().unwrap());;
        // assert_eq!(mounts.find(|m|
        //        m.contains(&MountInfoParam::Spec("rootfs"))
        //     ).unwrap(), mount_root.clone());
        // let mut mounts = MountInfoIter::new(buf.clone()).map(|m| m.ok().unwrap());;
        // assert_eq!(mounts.find(|m|
        //         m.contains(&MountInfoParam::MountPoint(Path::new("/")))
        //     ).unwrap(), mount_root.clone());
        // let mut mounts = MountInfoIter::new(buf.clone()).map(|m| m.ok().unwrap());;
        // assert_eq!(mounts.find(|m|
        //         m.contains(&MountInfoParam::VfsType("tmpfs"))
        //     ).unwrap(), mount_tmp.clone());
        let mut mounts = MountInfoIter::new(buf.clone()).map(|m| m.ok().unwrap());
        let mnt_ops = [MntOps::Write(true),
                       MntOps::Suid(false),
                       MntOps::Dev(false),
                       MntOps::Exec(false)];
        assert_eq!(mounts
                       .find(|m| {
                                 mnt_ops
                                     .iter()
                                     .all(|o| m.contains(&MountInfoParam::MntOps(o)))
                             })
                       .unwrap(),
                   mount_sysfs.clone());

        // let mounts = MountInfoIter::new(buf.clone()).map(|m| m.ok().unwrap());
        // assert_eq!(mounts.filter(|m|
        //          m.contains(&MountInfoParam::Freq(&DumpField::Ignore))
        //     ).collect::<Vec<_>>(), mounts_all.clone());
        // let mounts = MountInfoIter::new(buf.clone()).map(|m| m.ok().unwrap());
        // assert_eq!(mounts.filter(|m|
        //         m.contains(&MountInfoParam::PassNo(&None))
        //     ).collect::<Vec<_>>(), mounts_all.clone());
    }
}

//   ffcnt
//   Copyright (C) 2017 The 8472
//
//   This program is free software; you can redistribute it and/or modify
//   it under the terms of the GNU General Public License as published by
//   the Free Software Foundation; either version 3 of the License, or
//   (at your option) any later version.
//
//   This program is distributed in the hope that it will be useful,
//   but WITHOUT ANY WARRANTY; without even the implied warranty of
//   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//   GNU General Public License for more details.
//
//   You should have received a copy of the GNU General Public License
//   along with this program; if not, write to the Free Software Foundation,
//   Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301  USA
#![feature(alloc_system)]
extern crate alloc_system;
#[macro_use] extern crate clap;
#[macro_use] extern crate derive_error;
extern crate walkdir;
extern crate time;
extern crate btrfs2 as btrfs;
extern crate nix;

use btrfs::linux::{get_file_extent_map_for_path};
use std::error::Error;
use std::io::*;
use std::path::Path;
use clap::{Arg, App};
use walkdir::WalkDir;
use std::iter::Iterator;
use std::fs::*;
use time::precise_time_ns;
//use std::os::linux::fs::MetadataExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use nix::unistd::chown;


#[derive(Debug, Error)]
enum CliError {
    Io(std::io::Error)
}


fn recurse(p : &Path, preserve_ownership: bool, skip_contiguous: bool) -> Result<()> {

    for subdir in WalkDir::new(&p).min_depth(1).max_depth(1).into_iter().map(|dent| dent.unwrap())
        .filter(|dent| dent.file_type().is_dir()) {
        recurse(subdir.path(), preserve_ownership, skip_contiguous)?;
    }

    if skip_contiguous {
        match get_file_extent_map_for_path(&p) {
            Ok(extents) => {
                if extents.len() == 1 {
                    return Ok(());
                }
            }
            Err(_) => {}
        }
    }

    if p.parent() == None {
        writeln!(stderr(), "No parent directory; skipping {}", p.to_string_lossy())?;
        return Ok(())
    }

    let mut tmp = p.with_file_name(format!("{}.rebuild {}.tmp", p.file_name().unwrap().to_string_lossy(),precise_time_ns()));
    let src_attrs = p.metadata()?;

    match DirBuilder::new().mode(0o700).create(&tmp) {
        Ok(_) => {}
        Err(e) => {
            writeln!(stderr(), "{} - Could not create {}; skipping {}", e.description(), tmp.as_path().to_string_lossy(), p.to_string_lossy())?;
            return Ok(());
        }
    }

    let dst_attrs = tmp.metadata()?;

    if src_attrs.dev() != dst_attrs.dev() {
        writeln!(stderr(), "Skipping filesystem boundary {}", p.to_string_lossy())?;
        remove_dir(tmp)?;
        return Ok(());
    }

    if preserve_ownership && (src_attrs.uid() != dst_attrs.uid() || src_attrs.gid() != dst_attrs.gid()) {
        match chown(&tmp, Some(src_attrs.uid()), Some(src_attrs.gid())) {
            Ok(_) => {},
            Err(e) => {
                writeln!(stderr(), "{} - Cannot preserve ownership; skipping {}", e.description(), p.to_string_lossy())?;
                remove_dir(tmp)?;
                return Ok(());
            }
        }
    }

    // TODO: cleanup or error message about partial progress on error
    for child in WalkDir::new(&p).min_depth(1).max_depth(1).into_iter().map(|dent| dent.unwrap()) {
        let cp = child.path();
        tmp.push(cp.file_name().unwrap());
        rename(cp, &tmp)?;
        tmp.pop();
    }


    set_permissions(&tmp, src_attrs.permissions())?;

    // p should now be empty
    rename(&tmp, p)?;

    Ok(())
}

fn process_args() -> std::result::Result<(), CliError> {
    let matches = App::new("defrag directory tree by recursively moving files into new directories
    Does not preserve extended attributes (acl, xattr) or timestamps of directories.
    It is not atomic and will interfere with any process attempting to access the subtree
    ")
        .version(crate_version!())
        .arg(Arg::with_name("skip").short("c").long("check-frag").takes_value(false).help("check fragmentation status, skip if not fragmented"))
        .arg(Arg::with_name("take").short("t").long("take-ownership").takes_value(false).help("do not try to preserve uid/gid of directories"))
        .arg(Arg::with_name("dir").index(1).multiple(false).required(true).help("dir to rebuild"))
        .get_matches();

    let preserve = !matches.is_present("take");
    let skip = matches.is_present("skip");
    let dir = Path::new(&matches.value_of("dir").unwrap()).to_owned().canonicalize()?;

    recurse(&dir, preserve, skip)?;

    Ok(())
}


fn main() {

    match process_args() {
        Ok(_) => {
            std::process::exit(0);
        }
        Err(e) => {
            writeln!(std::io::stderr(),"{}", e.description()).unwrap();
            std::io::stderr().flush().unwrap();
            std::process::exit(1);
        }
    };
}
/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

mod alias;
pub(crate) mod create;
mod delete;
mod download_attachment;
pub(crate) mod find;
pub(crate) mod get_one;
pub(crate) mod item_keys;
pub(crate) mod list;
mod members;
mod r#move;
mod move_to_folder;
mod open;
#[allow(dead_code)]
mod revisions;
mod share;
mod trash;
mod update;

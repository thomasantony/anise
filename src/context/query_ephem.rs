/*
 * ANISE Toolkit
 * Copyright (C) 2021-2022 Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Documentation: https://nyxspace.com/
 */

use log::trace;

use crate::constants::celestial_objects::SOLAR_SYSTEM_BARYCENTER;
use crate::hifitime::Epoch;
use crate::math::Vector3;
use crate::{
    asn1::{context::AniseContext, ephemeris::Ephemeris},
    errors::{AniseError, IntegrityErrorKind},
    frame::Frame,
};

/// **Limitation:** no translation or rotation may have more than 8 nodes.
pub const MAX_TREE_DEPTH: usize = 8;

impl<'a> AniseContext<'a> {
    /// Try to return the ephemeris for the provided index, or returns an error.
    pub fn try_ephemeris_data(&self, idx: usize) -> Result<&'a Ephemeris, AniseError> {
        self.ephemeris_data
            .get(idx)
            .ok_or(AniseError::IntegrityError(IntegrityErrorKind::LookupTable))
    }

    /// Try to return the orientation for the provided index, or returns an error.
    pub fn try_orientation_data(&self, idx: usize) -> Result<&'a Ephemeris, AniseError> {
        self.orientation_data
            .get(idx)
            .ok_or(AniseError::IntegrityError(IntegrityErrorKind::LookupTable))
    }

    /// Try to construct the path from the source frame all the way to the solar system barycenter or to the root ephemeris of this context
    pub fn try_ephemeris_path(
        &self,
        source: &Frame,
    ) -> Result<(usize, [Option<u32>; MAX_TREE_DEPTH]), AniseError> {
        // Build a tree, set a fixed depth to avoid allocations
        let mut of_path = [None; MAX_TREE_DEPTH];
        let mut of_path_len = 0;
        let mut prev_ephem_hash = source.ephemeris_hash;
        for _ in 0..MAX_TREE_DEPTH {
            // The solar system barycenter has a hash of 0.
            // TODO: Find a way to specify the true root of the context -- maybe catch this err and set this as the root?
            let idx = self.ephemeris_lut.index_for_hash(&prev_ephem_hash)?;
            // let parent_hash = self.try_ephemeris_data(idx.into())?.parent_ephemeris_hash;
            let parent_ephem = self.try_ephemeris_data(idx.into())?;
            let parent_hash = parent_ephem.parent_ephemeris_hash;
            of_path[of_path_len] = Some(parent_hash);
            of_path_len += 1;
            if parent_hash == SOLAR_SYSTEM_BARYCENTER {
                return Ok((of_path_len, of_path));
            } else if let Err(e) = self.ephemeris_lut.index_for_hash(&parent_hash) {
                if e == AniseError::ItemNotFound {
                    // We have reached the root of this ephemeris and it has no parent.
                    trace!("{parent_hash} has no parent in this context");
                    return Ok((of_path_len, of_path));
                }
            }
            prev_ephem_hash = parent_hash;
        }
        Err(AniseError::MaxTreeDepth)
    }

    /// Returns the root of two frames. This may return a `DisjointRoots` error if the frames do not share a common root, which is considered a file integrity error.
    ///
    /// # Example
    ///
    /// If the "from" frame is _Earth Barycenter_ whose path to the ANISE root is the following:
    /// ```text
    /// Solar System barycenter
    /// ╰─> Earth Moon Barycenter
    ///     ╰─> Earth
    /// ```
    ///
    /// And the "to" frame is _Luna_, whose path is:
    /// ```text
    /// Solar System barycenter
    /// ╰─> Earth Moon Barycenter
    ///     ╰─> Luna
    ///         ╰─> LRO
    /// ```
    ///
    /// Then this function will return the common root/node as a hash, in this case, the hash of the "Earth Moon Barycenter".
    ///
    /// # Note
    /// A proper ANISE file should only have a single root and if two paths are empty, then they should be the same frame.
    /// If a DisjointRoots error is reported here, it means that the ANISE file is invalid.
    ///
    /// # Time complexity
    /// This can likely be simplified as this as a time complexity of O(n×m) where n, m are the lengths of the paths from
    /// the ephemeris up to the root.
    pub fn find_ephemeris_root(
        &self,
        from_frame: Frame,
        to_frame: Frame,
    ) -> Result<u32, AniseError> {
        if from_frame == to_frame {
            // Both frames match, return this frame's hash (i.e. no need to go higher up).
            return Ok(from_frame.ephemeris_hash);
        }

        // Grab the paths
        let (from_len, from_path) = self.try_ephemeris_path(&from_frame)?;
        let (to_len, to_path) = self.try_ephemeris_path(&to_frame)?;

        // Now that we have the paths, we can find the matching origin.

        // If either path is of zero length, that means one of them is at the root of this ANISE file, so the common
        // path is which brings the non zero-length path back to the file root.
        if from_len == 0 && to_len == 0 {
            Err(AniseError::IntegrityError(
                IntegrityErrorKind::DisjointRoots {
                    from_frame,
                    to_frame,
                },
            ))
        } else if from_len != 0 && to_len == 0 {
            // One has an empty path but not the other, so the root is at the empty path
            Ok(to_frame.ephemeris_hash)
        } else if to_len != 0 && from_len == 0 {
            // One has an empty path but not the other, so the root is at the empty path
            Ok(from_frame.ephemeris_hash)
        } else {
            // Either are at the ephemeris root, so we'll step through the paths until we find the common root.
            if from_len > to_len {
                // Iterate through the items in to_path because the longest path is necessarily includes in the shorter one,
                // so we can shrink the outer loop here
                for to_obj in to_path.iter().take(to_len) {
                    for from_obj in from_path.iter().take(from_len) {
                        if from_obj == to_obj {
                            // This is where the paths branch meet, so the root is the parent of the current item.
                            // Recall that the path is _from_ the source to the root of the context, so we're walking them
                            // backward until we find "where" the paths branched out.
                            return Ok(to_obj.unwrap());
                        }
                    }
                }
            } else {
                // Same algorithm as above, just flipped
                for from_obj in from_path.iter().take(from_len) {
                    for to_obj in to_path.iter().take(to_len) {
                        if from_obj == to_obj {
                            // This is where the paths branch meet, so the root is the parent of the current item.
                            // Recall that the path is _from_ the source to the root of the context, so we're walking them
                            // backward until we find "where" the paths branched out.
                            return Ok(to_obj.unwrap());
                        }
                    }
                }
            }
            // If the root is still unset, this is weird and I don't think it should happen, so let's raise an error.
            Err(AniseError::IntegrityError(IntegrityErrorKind::DataMissing))
        }
    }

    /// Returns the position vector and velocity vector needed to translate the `from_frame` to the `to_frame`.
    ///
    /// **WARNING:** This function only performs the translation and no rotation whatsoever. Use the `transform_from_to` function instead to include rotations.
    ///
    /// Note: this function performs a recursion of no more than twice the [MAX_TREE_DEPTH].
    pub fn translate_from_to(
        &self,
        from_frame: Frame,
        to_frame: Frame,
        epoch: Epoch,
    ) -> Result<(Vector3, Vector3), AniseError> {
        if from_frame == to_frame {
            // Both frames match, return this frame's hash (i.e. no need to go higher up).
            return Ok((Vector3::zeros(), Vector3::zeros()));
        }

        let ephem_root = self.find_ephemeris_root(from_frame, to_frame)?;
        // Now that we have the root, let's simply add the vectors from each frame to the root.

        let (pos_from_to_root, vel_from_to_root) =
            self.translate_from_to(from_frame, from_frame.with_ephem(ephem_root), epoch)?;

        let (pos_to_to_root, vel_to_to_root) =
            self.translate_from_to(to_frame, to_frame.with_ephem(ephem_root), epoch)?;

        // Return the difference of both vectors.
        Ok((
            pos_from_to_root - pos_to_to_root,
            vel_from_to_root - vel_to_to_root,
        ))
    }

    /// Translates a state with its origin (`to_frame`), returns that state with respect to the requested frame
    ///
    /// **WARNING:** This function only performs the translation and no rotation _whatsoever_. Use the `transform_state_to` function instead to include rotations.
    pub fn translate_state_to(
        &self,
        position_km: Vector3,
        velocity_kmps: Vector3,
        from_frame: Frame,
        to_frame: Frame,
        epoch: Epoch,
    ) -> Result<(Vector3, Vector3), AniseError> {
        // Compute the frame translation
        let (frame_pos, frame_vel) = self.translate_from_to(from_frame, to_frame, epoch)?;

        Ok((position_km + frame_pos, velocity_kmps + frame_vel))
    }
}

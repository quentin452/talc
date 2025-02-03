use std::borrow::Borrow;

use crate::bevy::prelude::*;

/// An axis-aligned bounding box, defined by:
/// - a center,
/// - the distances from the center to each faces along the axis,
///     the faces are orthogonal to the axis.
///
/// It is typically used as a component on an entity to represent the local space
/// occupied by this entity, with faces orthogonal to its local axis.
///
/// This component is notably used during "frustum culling", a process to determine
/// if an entity should be rendered by a [`Camera`] if its bounding box intersects
/// with the camera's [`Frustum`].
///
/// It will be added automatically by the systems in [`CalculateBounds`] to entities that:
/// - could be subject to frustum culling, for example with a [`Mesh3d`]
///     or `Sprite` component,
/// - don't have the [`NoFrustumCulling`] component.
///
/// It won't be updated automatically if the space occupied by the entity changes,
/// for example if the vertex positions of a [`Mesh3d`] are updated.
///
/// [`Camera`]: crate::camera::Camera
/// [`NoFrustumCulling`]: crate::view::visibility::NoFrustumCulling
/// [`CalculateBounds`]: crate::view::visibility::VisibilitySystems::CalculateBounds
/// [`Mesh3d`]: crate::mesh::Mesh
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct Aabb {
    pub center: Vec3,
    pub half_extents: Vec3,
}

impl Aabb {
    #[inline]
    pub fn from_min_max(minimum: Vec3, maximum: Vec3) -> Self {
        let center = 0.5 * (maximum + minimum);
        let half_extents = 0.5 * (maximum - minimum);
        Self {
            center,
            half_extents,
        }
    }

    /// Returns a bounding box enclosing the specified set of points.
    ///
    /// Returns `None` if the iterator is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{Vec3, Vec3};
    /// # use bevy_render::primitives::Aabb;
    /// let bb = Aabb::enclosing([Vec3::X, Vec3::Z * 2.0, Vec3::Y * -0.5]).unwrap();
    /// assert_eq!(bb.min(), Vec3::new(0.0, -0.5, 0.0));
    /// assert_eq!(bb.max(), Vec3::new(1.0, 0.0, 2.0));
    /// ```
    pub fn enclosing<T: Borrow<Vec3>>(iter: impl IntoIterator<Item = T>) -> Option<Self> {
        let mut iter = iter.into_iter().map(|p| *p.borrow());
        let mut min = iter.next()?;
        let mut max = min;
        for v in iter {
            min = Vec3::min(min, v);
            max = Vec3::max(max, v);
        }
        Some(Self::from_min_max(min, max))
    }

    #[inline]
    pub fn min(&self) -> Vec3 {
        self.center - self.half_extents
    }

    #[inline]
    pub fn max(&self) -> Vec3 {
        self.center + self.half_extents
    }
}

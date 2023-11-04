use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::{imports::*, VMAImage, VkInit};

impl VkInit {
    /// Utility function to recreate the swapchain, swapchain images and image views.
    ///
    /// Function waits for device_wait_idle before destroying the swapchain.
    /// Images must be transitioned to the appropriate image layout after recreation.

    pub fn on_resize<T: HasRawDisplayHandle + HasRawWindowHandle>(
        &mut self,
        window: &T,
        new_size: [u32; 2],
    ) -> Result<(), Error> {
        unsafe {
            let display_h = window.raw_display_handle();
            let window_h = window.raw_window_handle();

            let head = self.head.as_mut().unwrap();
            self.device.device_wait_idle()?;

            //destroy swapchain
            for image_view in &head.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            head.swapchain_loader
                .destroy_swapchain(head.swapchain, None);

            //Destroy depth image
            head.depth_image.destroy(&self.device, &self.allocator)?;

            //destroy surface
            head.surface_loader.destroy_surface(head.surface, None);

            //recreate surface
            let (surface_loader, surface, surface_info) = Self::create_surface(
                &self.entry,
                &self.instance,
                display_h,
                window_h,
                new_size,
                &self.physical_device,
                &self.create_info,
            )?;
            head.surface_loader = surface_loader;
            head.surface = surface;
            head.surface_info = surface_info;

            //recreate swapchain
            let (swapchain_loader, swapchain) = Self::create_swapchain(
                &self.instance,
                &self.device,
                &head.surface,
                &head.surface_info,
                new_size,
            )?;
            let (swapchain_images, swapchain_image_views) = Self::create_swapchain_images(
                &self.device,
                &swapchain_loader,
                &swapchain,
                &head.surface_info,
            )?;

            head.swapchain_loader = swapchain_loader;
            head.swapchain = swapchain;
            head.swapchain_images = swapchain_images;
            head.swapchain_image_views = swapchain_image_views;
            head.surface_info.current_extent = Extent2D {
                width: new_size[0],
                height: new_size[1],
            };

            //recreate depth image
            let extent = Extent3D {
                width: new_size[0],
                height: new_size[1],
                depth: 1,
            };
            head.depth_image = VMAImage::create_depth_image(
                &self.device,
                &self.allocator,
                extent,
                head.depth_format,
                head.depth_format_sizeof,
            )?;
        }

        Ok(())
    }
}

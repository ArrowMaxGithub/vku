use crate::{imports::*, VMAImage, VkInit};

impl VkInit {
    /// Utility function to recreate the swapchain, swapchain images and image views.
    ///
    /// Function waits for device_wait_idle before destroying the swapchain.

    pub fn on_resize(
        &mut self,
        display_handle: &RawDisplayHandle,
        window_handle: &RawWindowHandle,
        new_size: [u32; 2],
    ) -> Result<(), Error> {
        unsafe {
            let mut head = self.head.as_mut().unwrap();
            self.device.device_wait_idle()?;

            //destroy swapchain
            head.swapchain_loader
                .destroy_swapchain(head.swapchain, None);
            for image_view in &head.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            for image in &head.swapchain_images {
                self.device.destroy_image(*image, None);
            }

            //recreate depth image
            let extent = Extent3D {
                width: new_size[0],
                height: new_size[1],
                depth: 1,
            };
            let new_depth_img = VMAImage::create_depth_image(
                &self.device,
                &mut self.allocator,
                extent,
                Format::D32_SFLOAT,
                "VKU_DepthImage",
            )?;

            //destroy depth image
            let old_depth_img = std::mem::replace(&mut head.depth_image, new_depth_img);
            old_depth_img.destroy(&self.device, &mut self.allocator)?;

            //destroy surface
            head.surface_loader.destroy_surface(head.surface, None);

            //recreate surface
            let (surface_loader, surface, surface_info) = Self::create_surface(
                &self.entry,
                &self.instance,
                display_handle,
                window_handle,
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
        }

        Ok(())
    }
}

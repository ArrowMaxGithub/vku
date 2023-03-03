use crate::{imports::*, VkInit};

impl VkInit {
    /// Utility function to recreate the swapchain, swapchain images and image views.
    ///
    /// Function waits for device_wait_idle before destroying the swapchain.
    /// Images must be transitioned to the appropriate image layout after recreation.
    pub fn recreate_swapchain(&mut self, size: [u32; 2], frames_in_flight: u32) -> Result<()> {
        unsafe {
            let mut head = self.head.as_mut().unwrap();
            self.device.device_wait_idle()?;

            //destroy swapchain
            for image_view in &head.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            head.swapchain_loader
                .destroy_swapchain(head.swapchain, None);

            //recreate swapchain
            let (swapchain_loader, swapchain) = Self::create_swapchain(
                &self.instance,
                &self.device,
                &head.surface,
                &head.surface_info,
                size,
                frames_in_flight,
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
                width: size[0],
                height: size[1],
            };
        }

        Ok(())
    }
}

use crate::{imports::*, VkInit};

impl VkInit {
    /// Utility function to recreate the swapchain, swapchain images and image views.
    ///
    /// Function waits for device_wait_idle before destroying the swapchain.
    /// Images must be transitioned to the appropriate image layout after recreation.
    pub fn recreate_swapchain(&mut self, size: [u32; 2], frames_in_flight: u32) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()?;

            //destroy swapchain
            for image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);

            //recreate swapchain
            let window_extent = Extent2D {
                width: size[0],
                height: size[1],
            };
            let (swapchain_loader, swapchain) = Self::create_swapchain(
                &self.instance,
                &self.device,
                &self.surface,
                &self.info.surface_info,
                &window_extent,
                frames_in_flight,
            )?;
            let (swapchain_images, swapchain_image_views) = Self::create_swapchain_images(
                &self.device,
                &swapchain_loader,
                &swapchain,
                &self.info.surface_info,
            )?;

            self.swapchain_loader = swapchain_loader;
            self.swapchain = swapchain;
            self.swapchain_images = swapchain_images;
            self.swapchain_image_views = swapchain_image_views;
            self.info.surface_info.current_extent = Extent2D {
                width: size[0],
                height: size[1],
            };
        }

        Ok(())
    }
}

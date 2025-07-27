use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{create_info::WindowOptions, imports::*, VMAImage, VkInit};

impl<'a> VkInit<'a> {
    /// Utility function to recreate the swapchain, swapchain images and image views.
    ///
    /// Function waits for device_wait_idle before destroying the swapchain.
    /// Images must be transitioned to the appropriate image layout after recreation.

    pub fn on_resize<T: HasDisplayHandle + HasWindowHandle>(
        &mut self,
        window_options: WindowOptions<T>,
    ) -> Result<(), Error> {
        unsafe {
            trace!("Resizing swapchain");

            let Some(head) = self.head.as_mut() else {
                return Err(Error::HeadCallOnHeadlessInstance);
            };

            self.device.device_wait_idle()?;

            //destroy swapchain
            for image_view in &head.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.fn_loader
                .swapchain_device()?
                .destroy_swapchain(head.swapchain, None);

            //Destroy depth image
            head.depth_image
                .destroy(&self.device, &mut self.allocator)?;

            //destroy surface
            self.fn_loader
                .surface_instance()?
                .destroy_surface(head.surface, None);

            //recreate surface
            let (surface, surface_info) = Self::create_surface(
                &self.fn_loader,
                &self.entry,
                &self.instance,
                &self.physical_device,
                &self.create_info,
                &window_options,
            )?;
            head.surface = surface;
            head.surface_info = surface_info;

            //recreate swapchain
            let swapchain =
                Self::create_swapchain(&self.fn_loader, &head.surface, &head.surface_info)?;
            let (swapchain_images, swapchain_image_views) = Self::create_swapchain_images(
                &self.fn_loader,
                &self.device,
                &swapchain,
                &head.surface_info,
            )?;

            head.swapchain = swapchain;
            head.swapchain_images = swapchain_images;
            head.swapchain_image_views = swapchain_image_views;

            //recreate depth image
            head.depth_image = VMAImage::create_depth_image(
                &self.device,
                &mut self.allocator,
                &head.surface_info,
                head.depth_format,
                head.depth_format_sizeof,
            )?;
        }

        Ok(())
    }
}

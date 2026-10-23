//! Exact held message on the retained channel; no re-encoding or retry.
use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::installed::entry_material::held_wire::HeldRequest;
use zbus::export::futures_core::Stream;
impl FixedManagerChannel {
    pub(in super::super) fn deliver_held_once(&mut self, request: &mut HeldRequest) -> Result<()> {
        let result = (|| {
            self.revalidate()?;
            if !request.names_match(&self.manager, &self.client) {
                return Err(ChannelError::Context);
            }
            let op = Operation::new()?;
            self.inspect()?;
            self.runtime.block_on(op.bound(async {
                // Register on the SAME connection before the irreversible send.
                let mut stream = zbus::MessageStream::from(&self.connection);
                let serial = request.serial();
                let message = request.arm().map_err(|_| ChannelError::Context)?;
                self.connection
                    .send(message)
                    .await
                    .map_err(|_| ChannelError::Io)?;
                for _ in 0..16 {
                    let reply =
                        std::future::poll_fn(|cx| std::pin::Pin::new(&mut stream).poll_next(cx))
                            .await
                            .ok_or(ChannelError::Reply)?
                            .map_err(|_| ChannelError::Reply)?;
                    if reply.header().reply_serial() == Some(serial) {
                        request.observe(&reply).map_err(|_| ChannelError::Reply)?;
                        return Ok(());
                    }
                }
                Err(ChannelError::Reply)
            }))?;
            self.inspect()?;
            self.revalidate()
        })();
        if result.is_err() {
            request.refuse();
            self.refused = true;
        }
        result
    }
}

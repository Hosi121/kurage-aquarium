//! Two reusable light batches: workers prepare one while the consumer shades
//! and paints the other. Dropping the consumer disconnects both queues before
//! joining the producer, so sink failures cannot strand a blocked worker.
use crate::{
    Vec3,
    export::Result,
    scene::{LightWorkspace, Scene},
};
use rayon::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::sync_channel,
};

#[derive(Clone, Copy)]
pub struct RenderConfig {
    pub columns: usize,
    pub rows: usize,
    pub count: usize,
}

struct Batch {
    start: usize,
    len: usize,
    frames: Vec<Vec<Vec3>>,
}
impl Batch {
    fn new(size: usize) -> Self {
        Self {
            start: 0,
            len: 0,
            frames: (0..size).map(|_| Vec::new()).collect(),
        }
    }
}

pub fn for_each_frame<F>(
    scene: &Scene,
    pool: &rayon::ThreadPool,
    config: RenderConfig,
    running: &AtomicBool,
    initial: (LightWorkspace, Vec<Vec3>),
    mut consume: F,
) -> Result<()>
where
    F: FnMut(usize, &mut Vec<Vec3>) -> Result<()>,
{
    if config.count == 0 {
        return Ok(());
    }
    let bytes = config.columns * 2 * config.rows * 4 * std::mem::size_of::<Vec3>();
    let size = (32 * 1024 * 1024 / (2 * bytes))
        .max(1)
        .min(pool.current_num_threads())
        .min(config.count);
    std::thread::scope(|scope| {
        let (ready_send, ready_receive) = sync_channel::<Batch>(1);
        let (free_send, free_receive) = sync_channel::<Batch>(1);
        let producer = scope.spawn(move || -> Result<()> {
            let mut workspaces: Vec<_> = (0..size).map(|_| LightWorkspace::default()).collect();
            workspaces[0] = initial.0;
            let mut first = Batch::new(size);
            first.frames[0] = initial.1;
            let mut available = std::collections::VecDeque::from([first, Batch::new(size)]);
            for start in (0..config.count).step_by(size) {
                if !running.load(Ordering::Relaxed) {
                    return Err("export interrupted".into());
                }
                let mut batch = if let Some(batch) = available.pop_front() {
                    batch
                } else {
                    match free_receive.recv() {
                        Ok(batch) => batch,
                        Err(_) => return Ok(()),
                    }
                };
                batch.start = start;
                batch.len = size.min(config.count - start);
                pool.install(|| {
                    batch.frames[..batch.len]
                        .par_iter_mut()
                        .zip(workspaces.par_iter_mut())
                        .enumerate()
                        .for_each(|(offset, (pixels, workspace))| {
                            scene.light_frame_into(
                                config.columns,
                                config.rows,
                                (start + offset) as f64 / 30.,
                                false,
                                workspace,
                                pixels,
                            );
                        });
                });
                if ready_send.send(batch).is_err() {
                    return Ok(());
                }
            }
            Ok(())
        });
        // Own both endpoints in this closure. They must be dropped even when
        // consume returns early, before waiting for the producer to finish.
        let result = (move || -> Result<()> {
            let mut seen = 0;
            while seen < config.count {
                if !running.load(Ordering::Relaxed) {
                    return Err("export interrupted".into());
                }
                let mut batch = ready_receive
                    .recv()
                    .map_err(|_| "render pipeline stopped before completing its frames")?;
                for offset in 0..batch.len {
                    if !running.load(Ordering::Relaxed) {
                        return Err("export interrupted".into());
                    }
                    consume(batch.start + offset, &mut batch.frames[offset])?;
                }
                seen += batch.len;
                // The producer can already be finished when the final batch
                // returns; a disconnected recycler is normal in that case.
                let _ = free_send.send(batch);
            }
            Ok(())
        })();
        let produced = producer.join().map_err(|_| "render worker panicked");
        result?;
        produced??;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc::channel, time::Duration};

    #[test]
    fn pipeline_preserves_order_and_recycles_partial_batches() {
        let scene = Scene::single(Some(19.2));
        for threads in [1, 2, 4] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            let mut seen = 0;
            for_each_frame(
                &scene,
                &pool,
                RenderConfig {
                    columns: 8,
                    rows: 5,
                    count: 7,
                },
                &AtomicBool::new(true),
                (LightWorkspace::default(), Vec::new()),
                |i, pixels| {
                    assert_eq!(i, seen);
                    assert_eq!(*pixels, scene.light_frame(8, 5, i as f64 / 30., false));
                    // Consumers are allowed to mutate the light buffer before
                    // it is recycled. Old contents must not leak into frames.
                    pixels.fill([123.; 3]);
                    seen += 1;
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(seen, 7);
        }
    }

    #[test]
    fn sink_failure_and_cancellation_join_workers_without_deadlock() {
        for cancel in [false, true] {
            let (send, receive) = channel();
            std::thread::spawn(move || {
                let scene = Scene::single(Some(19.2));
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(2)
                    .build()
                    .unwrap();
                let running = AtomicBool::new(true);
                let result = for_each_frame(
                    &scene,
                    &pool,
                    RenderConfig {
                        columns: 8,
                        rows: 5,
                        count: 30,
                    },
                    &running,
                    (LightWorkspace::default(), Vec::new()),
                    |_, _| {
                        if cancel {
                            running.store(false, Ordering::Relaxed);
                            Ok(())
                        } else {
                            Err("fixture sink failure".into())
                        }
                    },
                );
                send.send(result.is_err()).unwrap();
            });
            assert!(
                receive
                    .recv_timeout(Duration::from_secs(10))
                    .expect("pipeline did not shut down")
            );
        }
    }
}

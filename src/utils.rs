use gfx_hal::Backend;
use prelude::*;

fn empty_buffer<B: Backend, Item>(
    device: &B::Device,
    memory_types: &[MemoryType],
    properties: Properties,
    usage: buffer::Usage,
    item_count: usize,
) -> (B::Buffer, B::Memory) {
    // NOTE: Change Vertex -> Item
    // NOTE: Weird issue with std -> ::std
    // NOTE: Use passed in usage/properties

    let item_count = item_count; // NOTE: Change
    let stride = ::std::mem::size_of::<Item>() as u64;
    let buffer_len = item_count as u64 * stride;
    let unbound_buffer = device.create_buffer(buffer_len, usage).unwrap();
    let req = device.get_buffer_requirements(&unbound_buffer);
    let upload_type = memory_types
        .iter()
        .enumerate()
        .position(|(id, ty)| req.type_mask & (1 << id) != 0 && ty.properties.contains(properties))
        .unwrap()
        .into();

    let buffer_memory = device.allocate_memory(upload_type, req.size).unwrap();
    let buffer = device
        .bind_buffer_memory(&buffer_memory, 0, unbound_buffer)
        .unwrap();

    // NOTE: Move buffer fill to another function

    (buffer, buffer_memory)
}

pub fn fill_buffer<B: Backend, Item: Copy>(
    device: &B::Device,
    buffer_memory: &mut B::Memory,
    items: &[Item],
) {
    // NOTE: MESH -> items
    // NOTE: Recalc buffer_len

    let stride = ::std::mem::size_of::<Item>() as u64;
    let buffer_len = items.len() as u64 * stride;

    let mut dest = device
        .acquire_mapping_writer::<Item>(&buffer_memory, 0..buffer_len)
        .unwrap();
    dest.copy_from_slice(items);
    device.release_mapping_writer(dest);
}

pub fn create_buffer<B: Backend, Item: Copy>(
    device: &B::Device,
    memory_types: &[MemoryType],
    properties: Properties,
    usage: buffer::Usage,
    items: &[Item],
) -> (B::Buffer, B::Memory) {
    let (empty_buffer, mut empty_buffer_memory) =
        empty_buffer::<B, Item>(device, memory_types, properties, usage, items.len());

    fill_buffer::<B, Item>(device, &mut empty_buffer_memory, items);

    (empty_buffer, empty_buffer_memory)
}

/// Reinterpret an instance of T as a slice of u32s that can be uploaded as push
/// constants.
pub fn push_constant_data<T>(data: &T) -> &[u32] {
    let size = push_constant_size::<T>();
    let ptr = data as *const T as *const u32;

    unsafe { ::std::slice::from_raw_parts(ptr, size) }
}

/// Determine the number of push constants required to store T.
/// Panics if T is not a multiple of 4 bytes - the size of a push constant.
pub fn push_constant_size<T>() -> usize {
    const PUSH_CONSTANT_SIZE: usize = ::std::mem::size_of::<u32>();
    let type_size = ::std::mem::size_of::<T>();

    // We want to ensure that the type we upload as a series of push constants
    // is actually representable as a series of u32 push constants.
    assert!(type_size % PUSH_CONSTANT_SIZE == 0);

    type_size / PUSH_CONSTANT_SIZE
}

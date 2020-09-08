use core::ops::Range;
/// Translation types.
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub enum Translation {
    Identity,
    Offset(usize),
}

/// Memory attributes.
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub enum MemAttributes {
    CacheableDRAM,
    Device,
}

/// Access permissions.
#[derive(Copy, Clone)]
pub enum AccessPermissions {
    KernelReadOnly,
    KernelReadWrite,
    UserReadOnly,
    UserReadWrite,
}

/// Collection of memory attributes.
#[allow(missing_docs)]
#[derive(Copy, Clone)]
pub struct AttributeFields {
    pub mem_attributes: MemAttributes,
    pub acc_perms: AccessPermissions,
    pub executable: bool,
}
impl AttributeFields {
    const fn new(mem_attributes: MemAttributes, acc_perms: AccessPermissions, executable: bool) -> Self {
        AttributeFields{mem_attributes,acc_perms, executable}
    }
}
impl core::default::Default for AttributeFields {
    fn default() -> Self {
        AttributeFields::new(MemAttributes::CacheableDRAM, AccessPermissions::KernelReadWrite, false)
    }
}

/// Descriptor for a memory range.
#[allow(missing_docs)]
pub struct RangeDescriptor {
    pub name: &'static str,
    pub virtual_range: fn() -> Range<usize>,
    pub translation: Translation,
    pub attribute_fields: AttributeFields,
}
impl RangeDescriptor {
    const fn new(name: &'static str, virtual_range: fn() -> Range<usize>, translation: Translation, attribute_fields: AttributeFields) -> Self {
        RangeDescriptor {name, virtual_range, translation, attribute_fields}
    }
}

const KERNEL_RW_ : AttributeFields = AttributeFields::new(MemAttributes::CacheableDRAM, AccessPermissions::KernelReadWrite, false);
const KERNEL_R_X : AttributeFields = AttributeFields::new(MemAttributes::CacheableDRAM, AccessPermissions::KernelReadOnly, true);
const USER_RW_: AttributeFields = AttributeFields::new(MemAttributes::CacheableDRAM, AccessPermissions::UserReadWrite, false);
const USER_R_X : AttributeFields = AttributeFields::new(MemAttributes::CacheableDRAM, AccessPermissions::UserReadOnly, true);
const DEVICE : AttributeFields = AttributeFields::new(MemAttributes::Device,AccessPermissions::UserReadWrite, false);

use crate::utils::binary_info::BinaryInfo;
pub const MEMORY_LAYOUT : [RangeDescriptor; 5] = [
    RangeDescriptor::new(
        "Init Stack",
        || {
            let binary_info = BinaryInfo::get();
            0..binary_info.binary.start
        },
        Translation::Identity,
        KERNEL_RW_,
    ),
    RangeDescriptor::new(
        "Static Kernel Data and Code",
        || {
            let binary_info = BinaryInfo::get();
            binary_info.read_only
        },
        Translation::Identity,
        KERNEL_R_X,
    ),
    RangeDescriptor::new(
        "Mutable Kernel Data",
        || {
            let binary_info = BinaryInfo::get();
            binary_info.read_write
        },
        Translation::Identity,
        KERNEL_RW_,
    ),
    RangeDescriptor::new(
        "Initial Kernel Heap",
        || {
            let binary_info = BinaryInfo::get();
            binary_info.heap
        },
        Translation::Identity,
        KERNEL_RW_,
    ),
    RangeDescriptor::new(
        "Initial Kernel Heap",
        || {
            let binary_info = BinaryInfo::get();
            binary_info.mmio
        },
        Translation::Identity,
        DEVICE,
    )
];
pub struct StreamingContext {
    offset: usize,
}

impl StreamingContext {
    pub fn new() -> Self {
        StreamingContext { offset: 0 }
    }

    pub fn advance(&mut self, size: usize) -> usize {
        let current_offset = self.offset;
        self.offset += size;
        current_offset
    }

    pub fn next_slice<'a>(&mut self, bytes: &'a [u8], size: usize) -> &'a [u8] {
        let offset = self.advance(size);
        &bytes[offset..offset + size]
    }

    pub fn next_slice_mut<'a>(&mut self, buffer: &'a mut [u8], size: usize) -> &'a mut [u8] {
        let offset = self.advance(size);
        &mut buffer[offset..offset + size]
    }
}

pub trait EncodeDecode: Sized {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext);
    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self;
    fn size() -> usize;

    fn encode(&self) -> Vec<u8> {
        let mut buffer = vec![0u8; Self::size()];
        let mut context = StreamingContext::new();
        self.encode_internal(&mut buffer, &mut context);
        buffer
    }

    fn encode_to_array<const N: usize>(&self) -> [u8; N] {
        let vec_encoded = self.encode();
        let mut array_encoded = [0u8; N];
        array_encoded.copy_from_slice(&vec_encoded);
        array_encoded
    }

    fn decode(bytes: &[u8]) -> Self {
        let mut context = StreamingContext::new();
        Self::decode_internal(bytes, &mut context)
    }
}

pub trait NestedEncodeDecode: EncodeDecode {
    fn encode_nested<T: EncodeDecode>(
        buffer: &mut [u8],
        context: &mut StreamingContext,
        items: &[T],
    ) {
        for item in items {
            item.encode_internal(buffer, context);
        }
    }

    fn decode_nested<T: EncodeDecode>(
        bytes: &[u8],
        items: &mut [T],
        context: &mut StreamingContext,
    ) {
        for item in items.iter_mut() {
            *item = T::decode_internal(bytes, context);
        }
    }
}

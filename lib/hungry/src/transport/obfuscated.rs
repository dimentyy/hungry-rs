use cipher::{KeyIvInit, StreamCipher};

use crate::transport::{
    IdentifiableTransport, Intermediate, Transport, TransportRead, TransportWrite, UnpackResult,
};

type Cipher = ctr::Ctr128BE<aes::Aes256>;

/// # Transport obfuscation
///
/// ---
///
/// <https://core.telegram.org/mtproto/mtproto-transports#transport-obfuscation>
pub struct Obfuscated<T: IdentifiableTransport> {
    inner: T,
    random: [u8; 64],
}

pub struct ObfuscatedRead<T: Transport> {
    tail: usize,
    inner: T::Read,
    cipher: Cipher,
}

pub struct ObfuscatedWrite<T: Transport> {
    inner: T::Write,
    cipher: Cipher,
}

impl<T: IdentifiableTransport> Obfuscated<T> {
    pub fn try_new(inner: T, random: [u8; 64]) -> Result<Self, ()> {
        if random[0] == 0xef {
            return Err(());
        }

        if [
            Intermediate::TRANSPORT_IDENTIFIER,
            [0xdd, 0xdd, 0xdd, 0xdd],
            *b"POST",
            *b"GET ",
            *b"HEAD",
        ]
        .contains(random[0..4].try_into().unwrap())
        {
            return Err(());
        }

        if random[4..8] == 0i32.to_le_bytes() {
            return Err(());
        }

        Ok(Self { inner, random })
    }
}

impl<T: IdentifiableTransport> crate::Sealed for Obfuscated<T> {}

impl<T: IdentifiableTransport> Transport for Obfuscated<T> {
    type Read = ObfuscatedRead<T>;
    type Write = ObfuscatedWrite<T>;

    const INIT_SIZE: usize = T::INIT_SIZE + 64;

    fn init(self, writer_buffer: &mut unbite::DynBuf) -> (Self::Read, Self::Write) {
        writer_buffer.extend_from_array(&self.random);

        let mut random_rev = [0; 48];

        for i in 0..48 {
            random_rev[i] = self.random[56 - i - 1];
        }

        let iv = random_rev[0..16].try_into().unwrap();
        let key = random_rev[16..48].try_into().unwrap();

        let mut w_cipher = Cipher::new(key, iv);

        let len = writer_buffer.len();

        let (r, w) = self.inner.init(writer_buffer);

        w_cipher.apply_keystream(&mut writer_buffer.as_mut_slice()[len..]);

        (
            ObfuscatedRead {
                tail: 0,
                inner: r,
                cipher: Cipher::new(key, iv),
            },
            ObfuscatedWrite {
                inner: w,
                cipher: w_cipher,
            },
        )
    }

    type Envelope = T::Envelope;

    #[inline]
    fn envelope(buffer: &mut unbite::DynBuf) -> Self::Envelope {
        T::envelope(buffer)
    }
}

impl<T: IdentifiableTransport> TransportRead for ObfuscatedRead<T> {
    type Transport = Obfuscated<T>;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult {
        assert!(buffer.len() >= self.tail);

        self.cipher.apply_keystream(&mut buffer[self.tail..]);

        self.tail = buffer.len();

        let unpack = self.inner.unpack(buffer);

        if let UnpackResult::Unpacked { offset, .. } = unpack {
            self.tail -= offset;
        }

        unpack
    }
}

impl<T: IdentifiableTransport> TransportWrite for ObfuscatedWrite<T> {
    type Transport = Obfuscated<T>;

    fn pack(&mut self, buffer: &mut unbite::DynBuf, envelope: T::Envelope) {
        self.inner.pack(buffer, envelope);
        self.cipher.apply_keystream(buffer.as_mut_slice());
    }
}

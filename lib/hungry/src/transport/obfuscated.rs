use cipher::{KeyIvInit, StreamCipher};

use unbite::DynBuf;

use crate::transport::{
    IdentifiableTransport, Transport, TransportInit, TransportRead, TransportWrite, UnpackResult,
};

type Cipher = ctr::Ctr128BE<aes::Aes256>;

pub struct Obfuscated<T: IdentifiableTransport>(pub T);

pub struct ObfuscatedRead<T: Transport> {
    tail: usize,
    inner: T::Read,
    cipher: Cipher,
}

pub struct ObfuscatedInit<T: Transport> {
    random: [u8; 64],
    inner: T::Init,
}

pub struct ObfuscatedWrite<T: Transport> {
    inner: T::Write,
    cipher: Cipher,
}

impl<T: IdentifiableTransport> crate::Sealed for Obfuscated<T> {}

impl<T: IdentifiableTransport> Transport for Obfuscated<T> {
    type Read = ObfuscatedRead<T>;
    type Init = ObfuscatedInit<T>;
    type Write = ObfuscatedWrite<T>;

    fn split(self) -> (Self::Read, Self::Init, Self::Write) {
        let (r, init, w) = self.0.split();

        let mut random = [0; 64];

        loop {
            getrandom::fill(&mut random).unwrap();

            // FIXME
            if !matches!(random[0..4], [0, 0, 0, 0] | [0xdd, 0xdd, 0xdd, 0xdd]) {
                break;
            }
        }

        let mut random_rev = [0; 48];

        for i in 0..48 {
            random_rev[i] = random[56 - i - 1];
        }

        let iv = random_rev[0..16].try_into().unwrap();
        let key = random_rev[16..48].try_into().unwrap();

        (
            ObfuscatedRead {
                tail: 0,
                inner: r,
                cipher: Cipher::new(key, iv),
            },
            ObfuscatedInit {
                random,
                inner: init,
            },
            ObfuscatedWrite {
                inner: w,
                cipher: Cipher::new(key, iv),
            },
        )
    }

    type Envelope = T::Envelope;

    fn envelope(buffer: &mut DynBuf) -> Self::Envelope {
        T::envelope(buffer)
    }
}

impl<T: IdentifiableTransport> TransportRead for ObfuscatedRead<T> {
    type Transport = Obfuscated<T>;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult {
        assert!(buffer.len() > self.tail);

        self.cipher.apply_keystream(&mut buffer[self.tail..]);

        self.tail = buffer.len();

        let unpack = self.inner.unpack(buffer);

        if let UnpackResult::Unpacked { offset, .. } = unpack {
            self.tail -= offset;
        }

        unpack
    }
}

impl<T: IdentifiableTransport> TransportInit for ObfuscatedInit<T> {
    type Transport = Obfuscated<T>;

    const SIZE: usize = 64 + T::Init::SIZE;

    fn init(self, write: &mut ObfuscatedWrite<T>, buffer: &mut unbite::DynBuf) {
        buffer.extend_from_array(&self.random);

        let len = buffer.len();

        self.inner.init(&mut write.inner, buffer);

        let buf = &mut buffer.as_mut_slice()[len..];

        write.cipher.apply_keystream(buf);
    }
}

impl<T: IdentifiableTransport> TransportWrite for ObfuscatedWrite<T> {
    type Transport = Obfuscated<T>;

    fn pack(&mut self, buffer: &mut unbite::DynBuf, envelope: T::Envelope) {
        self.inner.pack(buffer, envelope);
        self.cipher.apply_keystream(buffer.as_mut_slice());
    }
}

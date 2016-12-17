#![no_std]

pub trait FallibleStreamingIterator {
    type Item: ?Sized;
    type Error;

    fn advance(&mut self) -> Result<(), Self::Error>;

    fn get(&self) -> Option<&Self::Item>;

    #[inline]
    fn next(&mut self) -> Result<Option<&Self::Item>, Self::Error> {
        self.advance()?;
        Ok(self.get())
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    #[inline]
    fn all<F>(&mut self, mut f: F) -> Result<bool, Self::Error>
        where Self: Sized,
              F: FnMut(&Self::Item) -> bool
    {
        while let Some(e) = self.next()? {
            if !f(e) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[inline]
    fn any<F>(&mut self, mut f: F) -> Result<bool, Self::Error>
        where Self: Sized,
              F: FnMut(&Self::Item) -> bool
    {
        self.all(|e| !f(e)).map(|r| !r)
    }

    #[inline]
    fn by_ref(&mut self) -> &mut Self
        where Self: Sized
    {
        self
    }

    #[inline]
    fn count(mut self) -> Result<usize, Self::Error>
        where Self: Sized
    {
        let mut count = 0;
        while let Some(_) = self.next()? {
            count += 1;
        }
        Ok(count)
    }

    #[inline]
    fn filter<F>(self, f: F) -> Filter<Self, F>
        where Self: Sized,
              F: FnMut(&Self::Item) -> bool
    {
        Filter {
            it: self,
            f: f,
        }
    }

    #[inline]
    fn fuse(self) -> Fuse<Self>
        where Self: Sized
    {
        Fuse {
            it: self,
            state: FuseState::Start,
        }
    }

    #[inline]
    fn map<F, B>(self, f: F) -> Map<Self, F, B>
        where Self: Sized,
              F: FnMut(&Self::Item) -> B
    {
        Map {
            it: self,
            f: f,
            value: None,
        }
    }

    #[inline]
    fn map_ref<F, B: ?Sized>(self, f: F) -> MapRef<Self, F>
        where Self: Sized,
              F: Fn(&Self::Item) -> &B
    {
        MapRef {
            it: self,
            f: f,
        }
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Result<Option<&Self::Item>, Self::Error> {
        for _ in 0..n {
            self.advance()?;
            if let None = self.get() {
                return Ok(None);
            }
        }
        self.next()
    }
}

pub struct Filter<I, F> {
    it: I,
    f: F,
}

impl<I, F> FallibleStreamingIterator for Filter<I, F>
    where I: FallibleStreamingIterator,
          F: FnMut(&I::Item) -> bool
{
    type Item = I::Item;
    type Error = I::Error;

    #[inline]
    fn advance(&mut self) -> Result<(), I::Error> {
        while let Some(i) = self.it.next()? {
            if (self.f)(i) {
                break;
            }
        }
        Ok(())
    }

    #[inline]
    fn get(&self) -> Option<&I::Item> {
        self.it.get()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.it.size_hint().1)
    }
}

#[derive(Copy, Clone)]
enum FuseState {
    Start,
    Middle,
    End,
}

pub struct Fuse<I> {
    it: I,
    state: FuseState,
}

impl<I> FallibleStreamingIterator for Fuse<I>
    where I: FallibleStreamingIterator
{
    type Item = I::Item;
    type Error = I::Error;

    #[inline]
    fn advance(&mut self) -> Result<(), I::Error> {
        match self.state {
            FuseState::Start => {
                self.state = match self.it.next()? {
                    Some(_) => FuseState::Middle,
                    None => FuseState::End,
                };
            }
            FuseState::Middle => {
                if let None = self.it.next()? {
                    self.state = FuseState::End;
                }
            }
            FuseState::End => {},
        }
        Ok(())
    }

    #[inline]
    fn get(&self) -> Option<&I::Item> {
        match self.state {
            FuseState::Middle => self.it.get(),
            FuseState::Start | FuseState::End => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }

    #[inline]
    fn next(&mut self) -> Result<Option<&I::Item>, I::Error> {
        match self.state {
            FuseState::Start => {
                match self.it.next()? {
                    Some(v) => {
                        self.state = FuseState::Middle;
                        Ok(Some(v))
                    }
                    None => {
                        self.state = FuseState::End;
                        Ok(None)
                    }
                }
            }
            FuseState::Middle => {
                match self.it.next()? {
                    Some(v) => Ok(Some(v)),
                    None => {
                        self.state = FuseState::End;
                        Ok(None)
                    }
                }
            }
            FuseState::End => Ok(None)
        }
    }
}

pub struct Map<I, F, B>
{
    it: I,
    f: F,
    value: Option<B>,
}

impl<I, F, B> FallibleStreamingIterator for Map<I, F, B>
    where I: FallibleStreamingIterator,
          F: FnMut(&I::Item) -> B
{
    type Item = B;
    type Error = I::Error;

    #[inline]
    fn advance(&mut self) -> Result<(), I::Error> {
        self.value = self.it.next()?.map(&mut self.f);
        Ok(())
    }

    #[inline]
    fn get(&self) -> Option<&B> {
        self.value.as_ref()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
}

pub struct MapRef<I, F> {
    it: I,
    f: F,
}

impl<I, F, B: ?Sized> FallibleStreamingIterator for MapRef<I, F>
    where I: FallibleStreamingIterator,
          F: Fn(&I::Item) -> &B,
{
    type Item = B;
    type Error = I::Error;

    #[inline]
    fn advance(&mut self) -> Result<(), I::Error> {
        self.it.advance()
    }

    #[inline]
    fn get(&self) -> Option<&B> {
        self.it.get().map(&self.f)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn _is_object_safe(_: &FallibleStreamingIterator<Item = (), Error = ()>) {}
}

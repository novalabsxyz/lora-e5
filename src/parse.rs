use super::*;

impl<const N: usize> LoraE5<N> {
    pub(crate) fn read_until_break(&mut self, timeout: Duration) -> Result<usize> {
        self.read_until_pattern("\n", timeout)
    }

    pub(crate) fn read_until_pattern(&mut self, pattern: &str, timeout: Duration) -> Result<usize> {
        let mut cursor = 0;
        let mut time = time::Instant::now();
        loop {
            if let Ok(n) = self.port.read(&mut self.buf[cursor..]) {
                if n != 0 {
                    cursor += n;
                    time = time::Instant::now();
                }
            }

            if std::str::from_utf8(&self.buf[..cursor])?.ends_with(pattern) {
                return Ok(cursor);
            }

            if time.elapsed() > timeout {
                let partial_response = std::str::from_utf8(&self.buf[..cursor])?;
                return Err(Error::PartialResponse(partial_response.to_string()));
            }
        }
    }

    pub(crate) fn framed_response(&mut self, n: usize, expected_prelude: &str) -> Result<&str> {
        let response = std::str::from_utf8(&self.buf[..n])?;
        let (prelude, mode_response) = response.split_at(expected_prelude.len());
        if prelude == expected_prelude {
            Ok(mode_response)
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }

    pub(crate) fn check_framed_response(
        &mut self,
        n: usize,
        expected_prelude: &str,
        expected_response: &str,
    ) -> Result {
        let response = self.framed_response(n, expected_prelude)?;
        if response.trim_end() == expected_response {
            Ok(())
        } else {
            Err(Error::UnexpectedResponse(response.to_string()))
        }
    }
}

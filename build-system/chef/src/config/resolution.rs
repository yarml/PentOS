pub struct Resolution {
    pub xres: usize,
    pub yres: usize,
}

impl TryFrom<&str> for Resolution {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let Some((xres, yres)) = value.split_once('x') else {
            return Err(());
        };

        if xres.chars().any(|c| !c.is_numeric()) || yres.chars().any(|c| !c.is_numeric()) {
            return Err(());
        }

        let xres = xres.parse().map_err(|_| ())?;
        let yres = yres.parse().map_err(|_| ())?;
        Ok(Self { xres, yres })
    }
}

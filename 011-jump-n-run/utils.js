
const utils = {
    loadImages: sources => new Promise((resolve, reject) => {
        const images = {}
        const names = Object.keys(sources)
        let loaded = 0, failed = false
        const onload = () => {
            loaded += 1
            if (loaded == names.length && !failed)
                resolve(images)
        }, onerror = (err) => {
            failed = true
            reject(err)
        }

        for (let name in sources) {
            let img = new Image()
            img.src = sources[name]
            img.onload = onload
            img.onerror = onerror
            images[name] = img
        }
    }),

    animation: (images, delay = 1, scale = 3) => ({
        images: images,
        current: 0,
        delay: delay,
        delayCounter: 0,
        scale: scale,
        width: images[0].width * scale,
        height: images[0].height * scale,
        update: function(dt){
            this.delayCounter += 1
            if (this.delayCounter > this.delay) {
                this.delayCounter = 0
                this.current += 1
                if (this.current >= this.images.length)
                    this.current = 0
            }
        },
        renderTo: function(ctx, x, y){
            const img = this.images[this.current]
            const w = img.width * scale, h = img.height * this.scale
            ctx.drawImage(img, x - (w / 2), y - (h / 2), w, h)
        }
    }),

    sprite: (image, cw, ch, names) => ({
        image: image,
        cw: cw,
        ch: ch,
        names: names,
        renderTo: function(ctx, x, y, { x: sx, y: sy }, scale = 1){
            let w = this.cw * scale, h = this.ch * scale
            ctx.drawImage(this.image, sx, sy, this.cw, this.ch, x - (w / 2), y - (h / 2), w, h)
        }
    })
};

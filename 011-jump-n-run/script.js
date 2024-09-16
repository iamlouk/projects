"use strict";

const canvas = document.querySelector('canvas')
canvas.width = canvas.clientWidth
canvas.height = canvas.clientHeight
const ctx = canvas.getContext('2d')
ctx.font = '12px monospace'

let images = null
let runningMan = null
let jumpingMan = null

const update = (dt) => {
    background.update(dt)
    background.render(ctx)

    runningMan.update(dt)
    runningMan.renderTo(ctx, 100, 400)
}

const background = {
    move: true,
    speeds: [1, 1, 2, 3],
    offsets: [0, 0, 0, 0],
    update: function(dt) {
        if (!this.move)
            return

        for (let i = 0; i < 5; i++) {
            this.offsets[i] -= dt*this.speeds[i]
        }
    },
    render: function(ctx) {
        this.drawLayer(ctx, images.background1, this.offsets[0])
        this.drawLayer(ctx, images.background2, this.offsets[1])
        this.drawLayer(ctx, images.background3, this.offsets[2])
        this.drawLayer(ctx, images.background4, this.offsets[3])
    },
    drawLayer: function(ctx, img, offset) {
        const stretch = Math.round(canvas.height / img.height)
        const startX = offset % (img.width * stretch)
        for (let x = startX - (img.width * stretch); x < canvas.width; x += (img.width * stretch)) {
            ctx.drawImage(img, x, 0, img.width * stretch, img.height * stretch)
        }
    }
}

const start = async () => {

    images = await utils.loadImages({
        background1: './assets/jungle-pack/background/plx-2.png',
        background2: './assets/jungle-pack/background/plx-3.png',
        background3: './assets/jungle-pack/background/plx-4.png',
        background4: './assets/jungle-pack/background/plx-5.png',

        run0: './assets/jungle-pack/sprites-run/0.gif',
        run1: './assets/jungle-pack/sprites-run/1.gif',
        run2: './assets/jungle-pack/sprites-run/2.gif',
        run3: './assets/jungle-pack/sprites-run/3.gif',
        run4: './assets/jungle-pack/sprites-run/4.gif',
        run5: './assets/jungle-pack/sprites-run/5.gif',
        run6: './assets/jungle-pack/sprites-run/6.gif',
        run7: './assets/jungle-pack/sprites-run/7.gif',

        jump0: './assets/jungle-pack/sprites-jump/0.gif',

        maptiles: './assets/jungle-pack/tileset.png'
    })

    runningMan = utils.animation([
        images.run0, images.run1, images.run2, images.run3, images.run4, images.run5, images.run6, images.run7
    ], 3, 3)

    jumpingMan = utils.animation([
        images.jump0
    ])

    let prevtime = 0
    let frame = 0
    let fps = null
    window.requestAnimationFrame(function f(time) {
        const dt = time - prevtime
        prevtime = time
        ctx.fillStyle = '#ccffcc'
        ctx.fillRect(0, 0, canvas.width, canvas.height)
        update(Math.round(dt * 0.1))

        if (frame % 30 == 0 || dt > 100)
            fps = `FPS: ${Math.round(10000 / dt) / 10}`

        const pos = { x: 5, y: canvas.height - 10 }
        ctx.fillStyle = 'black'
        ctx.fillText(fps, pos.x, pos.y)
        frame += 1
        window.requestAnimationFrame(f)
    })
}

start()

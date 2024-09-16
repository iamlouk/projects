"use strict";

const canvas = document.querySelector('canvas')
canvas.width = canvas.clientWidth
canvas.height = canvas.clientHeight
const ctx = canvas.getContext('2d')
ctx.font = '12px monospace'

const STEPS_PER_RENDER = 15000
const root = new Vec2(canvas.width / 2, canvas.height / 2)
const vecs = [
    new Vec2(200, 0),
    new Vec2(50, 0),
]
const vecProps = [
    { rotationSpeed: 0.0001, x: 0, stretchSpeed: 0.002 },
    { rotationSpeed: 0.01,   x: 1, stretchSpeed: 0.003 },
]

let prevPos = null

ctx.strokeStyle = '#fff'
const update = (dt) => {
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.beginPath()
    if (prevPos != null)
        ctx.moveTo(prevPos.x, prevPos.y)

    for (let props of vecProps) {
        props.x += dt * props.stretchSpeed
    }    

    for (let i = 0; i < STEPS_PER_RENDER; i++) {
        let pos = new Vec2(root.x, root.y)
        for (let i = 0; i < vecs.length; i++) {
            let vec = vecs[i], props = vecProps[i]
            vec.rotateByAngle(dt * props.rotationSpeed)
            let stretch = Math.sin(props.x) + 1
            pos.x += vec.x * stretch
            pos.y += vec.y * stretch
        }

        if (prevPos == null) {
            prevPos = pos
            return
        }

        ctx.lineTo(pos.x, pos.y)
        prevPos = pos
    }
    ctx.stroke()
}



let prevtime = 0
let frame = 0
let fps = null
window.requestAnimationFrame(function f(time) {
    const dt = time - prevtime
    prevtime = time
    update(dt)

    if (frame % 30 == 0 || dt > 100)
        fps = `FPS: ${Math.round(10000 / dt) / 10}`

    const pos = { x: 5, y: canvas.height - 10 }
    ctx.clearRect(pos.x - 2, pos.y - 12, 75, 15)
    ctx.fillStyle = '#33cc33'
    ctx.fillText(fps, pos.x, pos.y)
    frame += 1
    window.requestAnimationFrame(f)
})


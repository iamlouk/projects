class Vec2 {
	constructor(x, y) {
		this.x = x
		this.y = y
	}

	get length(){
		return Math.sqrt(this.x * this.x + this.y * this.y)
	}

	rotateByAngle(a){
		let cos = Math.cos(a)
		let sin = Math.sin(a)
		let x = this.x * cos - this.y * sin
		let y = this.x * sin + this.y * cos
		this.x = x
		this.y = y
	}
}


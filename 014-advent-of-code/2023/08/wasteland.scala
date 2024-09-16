import scala.io.StdIn.readLine

enum Direction:
	case Left, Right

def readDirections(): List[Direction] =
	readLine().map(c => c match
		case 'L' => Direction.Left
		case 'R' => Direction.Right
		case _ => assert(false)).toList

class Node(val id: String, var left: Option[Node], var right: Option[Node]):
	override def toString(): String =
		id + " -> (" + left.map(x => x.id).getOrElse("???") + ", " +
					  right.map(x => x.id).getOrElse("???") + ")"

def getOrAdd(id: String, nodes: Map[String, Node]): (Node, Map[String, Node]) =
	nodes.get(id) match
		case Some(node) => (node, nodes)
		case None => {
			val node = Node(id, None, None)
			(node, nodes + (id -> node))
		}

def buildGraph(nodes0: Map[String, Node]): Map[String, Node] = readLine().toList match
	case List() => nodes0
	case List(n1, n2, n3, ' ', '=', ' ', '(', l1, l2, l3, ',', ' ', r1, r2, r3, ')') => {
		val (node,  nodes1) = getOrAdd(List(n1, n2, n3).mkString, nodes0)
		val (left,  nodes2) = getOrAdd(List(l1, l2, l3).mkString, nodes1)
		val (right, nodes3) = getOrAdd(List(r1, r2, r3).mkString, nodes2)
		assert(node.left.isEmpty && node.right.isEmpty)
		node.left = Some(left)
		node.right = Some(right)
		buildGraph(nodes3)
	}
	case _ => assert(false)

def walk(node: Node, nodes: Map[String, Node],
	     cycle: List[Direction], directions: List[Direction],
		 steps: Int): Int =
	node.id match
		case "ZZZ" => steps
		case _ => directions match
			case Nil => walk(node, nodes, cycle, cycle, steps)
			case Direction.Left  :: tail => walk(node.left.get,  nodes, cycle, tail, steps + 1)
			case Direction.Right :: tail => walk(node.right.get, nodes, cycle, tail, steps + 1)

@main def hello() = {
	val directions = readDirections()

	val empty = readLine()
	assert(empty.length == 0)

	val nodes = buildGraph(Map())
	val steps = walk(nodes.get("AAA").get, nodes, directions, directions, 0)

	println("Steps: ")
	println(steps)
}


#!/bin/bash

total_points=0

check_cards () {
	declare -A winning

	IFS=' '
	for card in $1 ; do
		winning[$card]=1
	done
	# echo "winning cards: ${!winning[@]}"

	local points=0
	for card in $2 ; do
		if [ "${winning[$card]}" = 1 ] ; then
			if [ $points = 0 ]; then
				points=1
			else
				points=$(($points * 2))
			fi
			# echo "winning card: $card (points: $points)"
		fi
	done

	total_points=$(($total_points + $points))
}

while IFS='' read -r line; do
	game=$(cut -d ":" -f 1 <<< $line)
	line=$(cut -d ":" -f 2 <<< $line)
	winning=$(cut -d "|" -f 1 <<< $line)
	cards=$(cut -d "|" -f 2 <<< $line)

	check_cards "$winning" "$cards"
done

echo "Total Points: " $total_points


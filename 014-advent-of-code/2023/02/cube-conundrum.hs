import System.IO
import System.Exit
import Debug.Trace
import qualified Data.Text as Text

maxReds = 12
maxGreens = 13
maxBlues = 14

toInt :: Text.Text -> Int
toInt txt = read (Text.unpack txt)

checkNumberOfCards :: (Int, Text.Text) -> Bool
checkNumberOfCards (count, color) = case (Text.unpack color) of
  "red" -> (count <= maxReds)
  "green" -> (count <= maxGreens)
  "blue" -> (count <= maxBlues)

checkGame :: Text.Text -> Bool
checkGame line =
  let cards = map (Text.words) (Text.splitOn (Text.pack ",") line) in
  all checkNumberOfCards (map (\x -> (toInt (head x), (head (tail x)))) cards)

checkLine :: Text.Text -> Int
checkLine line =
  let gameid = toInt (Text.drop 5 (head (Text.splitOn (Text.pack ":") line))) in
  let games = Text.splitOn (Text.pack ";") (head (tail (Text.splitOn (Text.pack ":") line))) in
  if (all checkGame games) then gameid else 0

readLines :: Int -> IO Int
readLines idsum = do
  isClosed <- isEOF
  if isClosed
    then return idsum
    else
      do
        line <- getLine
        readLines (idsum + (checkLine (Text.pack line)))

main :: IO()
main = do
  result <- readLines 0
  print result




(require '[clojure.string :as str])

(defn prediction [input]
  (let [diffs (map #(- %2 %1) input (rest input))]
    (if (every? #(= 0 %1) diffs)
      (last input)
      (+ (last input) (prediction diffs)))))

; (doseq [ln (line-seq (java.io.BufferedReader. *in*))]
;   (let [input (map #(Integer/parseInt %) (str/split ln #" "))]
;     (println input "->" (prediction input))))

(println "The result is: "
  (let [lines (line-seq (java.io.BufferedReader. *in*))
        prepline (fn [line] (map #(Integer/parseInt %) (str/split line #" ")))]
    (reduce + (map #(prediction (prepline %1)) lines))))

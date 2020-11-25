external test_func_1: int32 array -> int -> int32 = "test_func_1"

let%test "test func 1" =
  Util.check_leaks (fun () ->
    test_func_1 [|1l; 2l; 3l|] 0 = 1l &&
    test_func_1 [|1l; 2l; 3l|] 1 = 2l &&
    test_func_1 [|1l; 2l; 3l|] 2 = 3l &&
    test_func_1 [| 0l; |] 0 < 1l
  )

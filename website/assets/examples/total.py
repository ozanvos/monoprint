def total(cart, rate=0.1):
    subtotal = sum(
        price * qty
        for price, qty in cart
    )

    savings = subtotal * rate
    return subtotal - savings

cart = [(24, 2), (8, 3)]
print(total(cart))
